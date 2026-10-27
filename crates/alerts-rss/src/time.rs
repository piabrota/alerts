//! Feed times. RSS 2.0 uses RFC 822 (`Tue, 27 Oct 2026 03:00:00 GMT`), Atom uses RFC 3339
//! (`2026-10-27T03:00:00Z`). An [`Item`](crate::Item) keeps one form only: UTC as
//! `YYYY-MM-DDThh:mm:ss+00:00`, the form that CAP 1.2 and `alerts-feed` accept. In that form
//! the text order is the time order.

/// A feed time in the UTC form, or `None` when the text is not RFC 3339 or RFC 822.
pub fn normalize(s: &str) -> Option<String> {
    let s = s.trim();
    parse_rfc3339(s)
        .or_else(|| parse_rfc822(s))
        .map(rfc3339_utc)
}

/// Seconds since 1970-01-01T00:00:00Z as `YYYY-MM-DDThh:mm:ss+00:00`.
pub fn rfc3339_utc(secs: i64) -> String {
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let (h, mi, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}+00:00")
}

/// Days since 1970-01-01 of a civil date (the inverse of the algorithm above).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn epoch(date: (i64, i64, i64), time: (i64, i64, i64), offset_min: i64) -> Option<i64> {
    let (y, m, d) = date;
    let (h, mi, s) = time;
    let ok = (1..=12).contains(&m)
        && (1..=31).contains(&d)
        && (0..=23).contains(&h)
        && (0..=59).contains(&mi)
        && (0..=60).contains(&s);
    ok.then(|| days_from_civil(y, m, d) * 86_400 + h * 3600 + mi * 60 + s - offset_min * 60)
}

fn num(s: &str) -> Option<i64> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

/// `YYYY-MM-DDThh:mm:ss[.fff](Z|±hh:mm)`.
fn parse_rfc3339(s: &str) -> Option<i64> {
    let (date, rest) = s.split_once(['T', 't', ' '])?;
    let mut d = date.split('-');
    let ymd = (num(d.next()?)?, num(d.next()?)?, num(d.next()?)?);
    if d.next().is_some() || rest.len() < 8 {
        return None;
    }
    let (clock, zone) = rest.split_at(8);
    let mut c = clock.split(':');
    let hms = (num(c.next()?)?, num(c.next()?)?, num(c.next()?)?);
    let zone = match zone.strip_prefix('.') {
        Some(frac) => frac.trim_start_matches(|c: char| c.is_ascii_digit()),
        None => zone,
    };
    let offset = match zone {
        "Z" | "z" => 0,
        z => {
            let (sign, hm) = (z.get(..1)?, z.get(1..)?);
            let (h, m) = hm.split_once(':')?;
            let off = num(h)? * 60 + num(m)?;
            match sign {
                "+" => off,
                "-" => -off,
                _ => return None,
            }
        }
    };
    epoch(ymd, hms, offset)
}

/// `[Day, ]DD Mon YYYY hh:mm[:ss] zone`, with a two-digit year allowed by RFC 822.
fn parse_rfc822(s: &str) -> Option<i64> {
    let s = s.split_once(',').map_or(s, |(_, rest)| rest);
    let mut w = s.split_whitespace();
    let d = num(w.next()?)?;
    let m = match w.next()?.get(..3)?.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };
    let y = match num(w.next()?)? {
        y @ 0..50 => 2000 + y,
        y @ 50..100 => 1900 + y,
        y => y,
    };
    let mut c = w.next()?.split(':');
    let hms = (
        num(c.next()?)?,
        num(c.next()?)?,
        c.next().map_or(Some(0), num)?,
    );
    let offset = match w.next().unwrap_or("GMT") {
        "GMT" | "UT" | "UTC" | "Z" => 0,
        "EDT" => -4 * 60,
        "EST" | "CDT" => -5 * 60,
        "CST" | "MDT" => -6 * 60,
        "MST" | "PDT" => -7 * 60,
        "PST" => -8 * 60,
        z => {
            let (sign, hm) = (z.get(..1)?, z.get(1..)?);
            if hm.len() != 4 {
                return None;
            }
            let off = num(&hm[..2])? * 60 + num(&hm[2..])?;
            match sign {
                "+" => off,
                "-" => -off,
                _ => return None,
            }
        }
    };
    epoch((y, m, d), hms, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00+00:00");
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00+00:00");
        assert_eq!(rfc3339_utc(1_792_627_200), "2026-10-22T00:00:00+00:00");
        assert_eq!(days_from_civil(2000, 2, 29) * 86_400, 951_782_400);
    }

    #[test]
    fn both_forms_become_utc() {
        let utc = Some("2026-10-22T15:00:00+00:00".to_owned());
        assert_eq!(normalize("2026-10-22T12:00:00-03:00"), utc);
        assert_eq!(normalize("2026-10-22T15:00:00Z"), utc);
        assert_eq!(normalize("2026-10-22T15:00:00.123Z"), utc);
        assert_eq!(normalize("Thu, 22 Oct 2026 15:00:00 GMT"), utc);
        assert_eq!(normalize("22 Oct 2026 12:00:00 -0300"), utc);
        assert_eq!(normalize("Thu, 22 Oct 26 11:00 EDT"), utc);
    }

    #[test]
    fn bad_times_are_none() {
        assert_eq!(normalize(""), None);
        assert_eq!(normalize("ontem"), None);
        assert_eq!(normalize("2026-13-01T00:00:00Z"), None);
        assert_eq!(normalize("Thu, 22 Foo 2026 15:00:00 GMT"), None);
        assert_eq!(normalize("2026-10-22T15:00:00"), None);
    }
}

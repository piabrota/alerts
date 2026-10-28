//! The Marola Discord delivery test: one test alert reaches the official Marola Discord server.
//! It is a hard rule of the repository and part of `just check`. It needs the webhook of the
//! server in `MAROLA_DISCORD_WEBHOOK_URL` (from the environment or from `.env`), which never enters
//! the repository.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[tokio::test(flavor = "current_thread")]
#[ignore = "sends a message to Discord: run it with `just marola-discord`"]
async fn delivers_a_test_alert_to_the_marola_discord() {
    alerts::load_dotenv().expect("read .env");
    let webhook = alerts::env("MAROLA_DISCORD_WEBHOOK_URL")
        .expect("set MAROLA_DISCORD_WEBHOOK_URL to the webhook of the official Marola Discord");
    assert!(
        alerts::is_discord_webhook(&webhook),
        "MAROLA_DISCORD_WEBHOOK_URL is not a Discord webhook URL"
    );
    let alert = alerts::test_alert(&alerts::now_utc());
    let id = alerts::post(&webhook, &alert)
        .await
        .unwrap_or_else(|e| panic!("the Marola Discord did not store the test alert: {e}"));
    println!("marola-discord: test alert stored as message {id}");
}

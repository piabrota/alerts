// WeatherDisaster: the bridge between the Obsidian API and the plugin logic, which is Rust
// (crate alerts-obsidian) built to WebAssembly. This file only does what needs the Obsidian API:
// HTTP, vault files, notices, the status bar and the settings. `just obsidian` puts the
// wasm-bindgen glue (`wasm_bindgen`) and the WebAssembly (`WASM_BASE64`) before this file.

const {
  MarkdownRenderChild,
  MarkdownRenderer,
  Notice,
  Plugin,
  PluginSettingTab,
  Setting,
  TFile,
  normalizePath,
  requestUrl,
} = require("obsidian");

// The plugin keeps this many applied alerts and fetched CAP links in its data.
const KEEP = 5000;
// More new notes than this in one read give one summary notice.
const NOTICES = 3;

const message = (e) => (e instanceof Error ? e.message : String(e));

module.exports = class WeatherDisaster extends Plugin {
  async onload() {
    const bytes = Uint8Array.from(atob(WASM_BASE64), (c) => c.charCodeAt(0));
    await wasm_bindgen({ module_or_path: bytes });
    this.core = wasm_bindgen;
    const data = (await this.loadData()) ?? {};
    this.settings = Object.assign(JSON.parse(this.core.default_settings()), data.settings);
    // CAP identifier -> note path, for an Update or a Cancel of an earlier alert.
    this.index = data.index ?? {};
    // `identifier|sent` of each alert already applied, and each CAP link already fetched.
    this.seen = new Set(data.seen ?? []);
    this.links = new Set(data.links ?? []);
    this.poll = { feedUrl: this.settings.feedUrl, okAt: "", error: "" };
    this.blocks = new Set();
    this.statusBar = this.addStatusBarItem();
    this.addSettingTab(new SettingsTab(this.app, this));
    this.addCommand({ id: "read-feed", name: "Ler o feed agora", callback: () => this.read() });
    this.registerMarkdownCodeBlockProcessor("alerts", (_source, el, ctx) => {
      const block = { el, ctx };
      this.blocks.add(block);
      this.renderBlock(block);
    });
    this.registerEvent(this.app.metadataCache.on("resolved", () => this.refresh()));
    this.app.workspace.onLayoutReady(() => this.read());
    this.schedule();
    this.refresh();
  }

  schedule() {
    if (this.timer !== undefined) window.clearInterval(this.timer);
    const minutes = Math.max(1, Number(this.settings.pollMinutes) || 5);
    this.timer = this.registerInterval(window.setInterval(() => this.read(), minutes * 60000));
  }

  async get(url) {
    const r = await requestUrl({ url, headers: { "User-Agent": this.core.user_agent() }, throw: false });
    if (r.status !== 200) throw new Error(`HTTP ${r.status} em ${url}`);
    return r.text;
  }

  // One read of the feed. Any failure reaches the user: a notice, the status bar and the
  // `alerts` blocks. A failed read never shows as "no alert".
  async read() {
    if (this.busy) return;
    this.busy = true;
    const url = this.settings.feedUrl;
    this.poll.feedUrl = url;
    const notices = [];
    try {
      const feed = JSON.parse(this.core.parse_feed(await this.get(url)));
      const problems = feed.unreadable.map((e) => `uma entrada sem CAP (${e})`);
      for (const link of feed.links.filter((l) => !this.links.has(l))) {
        try {
          feed.alerts.push(JSON.parse(this.core.parse_cap(await this.get(link))));
          this.links.add(link);
        } catch (e) {
          problems.push(message(e));
        }
      }
      for (const alert of feed.alerts) {
        const notice = await this.take(alert);
        if (notice) notices.push(notice);
      }
      if (problems.length > 0) {
        throw new Error(`${problems.length} entrada(s) não lida(s), a primeira: ${problems[0]}`);
      }
      this.poll.okAt = new Date().toLocaleString("pt-BR");
      this.poll.error = "";
    } catch (e) {
      const error = message(e);
      if (error !== this.poll.error) new Notice(`Alertas: a leitura de ${url} falhou: ${error}`, 0);
      console.error("WeatherDisaster:", e);
      this.poll.error = error;
    } finally {
      for (const n of notices.slice(0, NOTICES)) new Notice(n, 10000);
      if (notices.length > NOTICES) new Notice(`Alertas: mais ${notices.length - NOTICES} notas de alerta.`, 10000);
      await this.save();
      this.busy = false;
      this.refresh();
    }
  }

  file(path) {
    const f = this.app.vault.getAbstractFileByPath(path);
    return f instanceof TFile ? f : undefined;
  }

  // Applies one alert to its note and returns the notice to show, if any.
  async take(alert) {
    const mark = `${alert.identifier}|${alert.sent}`;
    if (this.seen.has(mark)) return undefined;
    const json = JSON.stringify(alert);
    const keys = JSON.parse(this.core.note_keys(json));
    const own = (k) => normalizePath(this.core.note_path(this.settings.folder, k));
    const path =
      keys.map((k) => this.index[k]).find((p) => p && this.file(p)) ??
      keys.map(own).find((p) => this.file(p)) ??
      own(alert.identifier);
    const file = this.file(path);
    const existing = file ? await this.app.vault.read(file) : undefined;
    const out = JSON.parse(this.core.apply(json, JSON.stringify(this.settings), existing));
    this.seen.add(mark);
    if (out.action === "conflict") {
      new Notice(`Alertas: ${path} não foi alterada: ${out.reason}`, 0);
      return undefined;
    }
    if (out.action !== "write") return undefined;
    if (file) {
      await this.app.vault.modify(file, out.text);
    } else {
      const dir = path.split("/").slice(0, -1).join("/");
      if (dir && !this.app.vault.getAbstractFileByPath(dir)) await this.app.vault.createFolder(dir);
      await this.app.vault.create(path, out.text);
    }
    for (const k of keys) this.index[k] = path;
    return out.notice;
  }

  view() {
    const prefix = `${normalizePath(this.settings.folder)}/`;
    const notes = this.app.vault
      .getMarkdownFiles()
      .filter((f) => f.path.startsWith(prefix))
      .map((f) => ({ path: f.path, frontmatter: this.app.metadataCache.getFileCache(f)?.frontmatter ?? null }));
    return JSON.parse(this.core.active_view(JSON.stringify(notes), JSON.stringify(this.poll), Date.now() / 1000));
  }

  refresh() {
    const v = this.view();
    this.statusBar.setText(v.status);
    for (const block of this.blocks) {
      if (block.el.isConnected) this.renderBlock(block, v);
      else this.blocks.delete(block);
    }
  }

  renderBlock({ el, ctx }, v = this.view()) {
    el.empty();
    const child = new MarkdownRenderChild(el);
    ctx.addChild(child);
    MarkdownRenderer.render(this.app, v.markdown, el, ctx.sourcePath, child);
  }

  async save() {
    await this.saveData({
      settings: this.settings,
      index: this.index,
      seen: [...this.seen].slice(-KEEP),
      links: [...this.links].slice(-KEEP),
    });
  }

  // A new setting can keep an alert that a filter dropped before, so every alert is read again.
  async changeSettings(change) {
    Object.assign(this.settings, change);
    this.seen.clear();
    await this.save();
    this.schedule();
  }
};

const list = (s) =>
  s
    .split(",")
    .map((x) => x.trim())
    .filter((x) => x.length > 0);

class SettingsTab extends PluginSettingTab {
  constructor(app, plugin) {
    super(app, plugin);
    this.plugin = plugin;
  }

  display() {
    const { containerEl } = this;
    const s = this.plugin.settings;
    const set = (change) => this.plugin.changeSettings(change);
    containerEl.empty();
    new Setting(containerEl)
      .setName("Feed")
      .setDesc("Um feed Atom ou RSS de alertas CAP 1.2, ou um documento CAP.")
      .addText((t) => t.setValue(s.feedUrl).onChange((v) => set({ feedUrl: v.trim() })));
    new Setting(containerEl)
      .setName("Pasta")
      .setDesc("A pasta das notas de alerta, uma nota por alerta.")
      .addText((t) => t.setValue(s.folder).onChange((v) => set({ folder: v.trim() || "Alertas" })));
    new Setting(containerEl)
      .setName("Intervalo (minutos)")
      .addText((t) => t.setValue(String(s.pollMinutes)).onChange((v) => set({ pollMinutes: Number(v) || 5 })));
    new Setting(containerEl)
      .setName("UFs")
      .setDesc("Siglas separadas por vírgula, por exemplo PR, SC, RS. Vazio com municípios vazio: todas as áreas.")
      .addText((t) => t.setValue(s.ufs.join(", ")).onChange((v) => set({ ufs: list(v).map((u) => u.toUpperCase()) })));
    new Setting(containerEl)
      .setName("Municípios")
      .setDesc("Códigos IBGE de 7 dígitos, separados por vírgula.")
      .addText((t) => t.setValue(s.municipalities.join(", ")).onChange((v) => set({ municipalities: list(v) })));
    new Setting(containerEl)
      .setName("Severidade mínima")
      .addDropdown((d) =>
        d
          .addOptions({ Unknown: "todas", Minor: "menor", Moderate: "moderada", Severe: "severa", Extreme: "extrema" })
          .setValue(s.minSeverity)
          .onChange((v) => set({ minSeverity: v })),
      );
    new Setting(containerEl)
      .setName("Alertas de teste")
      .setDesc("Grava também os alertas com status CAP Test, para testar o feed.")
      .addToggle((t) => t.setValue(s.includeTests).onChange((v) => set({ includeTests: v })));
  }
}

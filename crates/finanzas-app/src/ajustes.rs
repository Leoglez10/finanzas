//! Ajustes: reminder preferences and the one-time importers.
use super::*;
use finanzas_core::agents::{self, Agent};
use finanzas_core::store::ImportSummary;
use overview::{card, muted};
use rune::FileDialogOptions;
use schedule_sheet::toggle;
use sheet::{button, field};
use theme::{badge, header_bar, headline, icon_title, titles};

pub struct ImportRequest { pub kind: data::ImportKind, pub source: std::path::PathBuf }
pub type ImportOutcome = Result<ImportSummary, String>;

fn count(n: usize, one: &str, many: &str) -> String { format!("{n} {}", if n == 1 { one } else { many }) }
/// "Importación completa: 1 cuenta, 2 categorías, … y 1 presupuesto."
pub fn summary_text(r: &ImportSummary) -> String {
    format!("Importación completa: {}, {}, {}, {}, {} y {}.", count(r.accounts, "cuenta", "cuentas"), count(r.categories, "categoría", "categorías"), count(r.payees, "comercio", "comercios"),
        count(r.entries, "movimiento", "movimientos"), count(r.schedules, "pago programado", "pagos programados"), count(r.budgets, "presupuesto", "presupuestos"))
}

/// Opens the file dialog for `kind`; choosing a file only asks for confirmation, it imports nothing.
pub fn choose(s: &mut State, tx: &mut Tx, kind: data::ImportKind) {
    if s.mutation_active() || s.import_confirm.is_some() { return; }
    let options = match kind {
        data::ImportKind::SwiftData => FileDialogOptions { title: "Selecciona el archivo Finanzas.store".into(), ..Default::default() },
        data::ImportKind::Actual => FileDialogOptions { title: "Selecciona el db.sqlite o el respaldo .zip de Actual".into(), ..Default::default() }.filter("Actual", &["sqlite", "zip", "blob"]),
    };
    s.import_outcome = None;
    tx.open_file(options, move |s: &mut State, result, _| match result {
        Ok(Some(source)) if !s.mutation_active() => s.import_confirm = Some(ImportRequest { kind, source }),
        Ok(_) => {}
        Err(error) => s.import_outcome = Some(Err(format!("No se pudo abrir el selector de archivos: {error}"))),
    });
}

pub fn confirm(s: &State, p: Palette) -> El {
    let request = s.import_confirm.as_ref().expect("import confirmation present");
    let name = match request.kind { data::ImportKind::SwiftData => "Finanzas (SwiftData)", data::ImportKind::Actual => "Actual" };
    let content = col().w(520.0).padding(24.0).gap(14.0).radius(18.0).fill(p.card)
        .child(text(format!("¿Importar desde {name}?")).font(type_::TITLE).color(p.ink))
        .child(text(format!("Origen: {}", request.source.display())).key("import-confirm-source").font(type_::BODY).color(p.ink).wrap_anywhere().w_full())
        .child(text(format!("Destino: {}", s.db_label)).key("import-confirm-destination").font(type_::BODY).color(p.ink).wrap_anywhere().w_full())
        .child(text("El archivo de origen solo se lee, nunca se modifica. La importación solo funciona si la base de datos de destino está vacía.").font(type_::BODY).color(p.muted).wrap().w_full())
        .child(row().wrap().w_full().justify_between().child(button("Cancelar", "import-cancel", p).on_press(|s: &mut State| s.import_confirm = None))
            .child(button("Importar", "import-confirm", p).fill(p.petroleum).default_button().on_press(|s: &mut State| s.run_import())));
    el().key("confirm-import").paint_order(100).raise().absolute(0.0, 0.0).wrap().w_full().h_full().center().fill(Color::BLACK.alpha(0.4)).modal(false, |s: &mut State| s.import_confirm = None).child(content)
}

type Step = (&'static str, fn(&mut State));
fn stepper(text_key: &'static str, caption: String, minus: Step, plus: Step, on: bool, p: Palette) -> El {
    let ink = if on { p.ink } else { p.muted };
    let step = |name: &'static str, (key, action): Step, spoken: &'static str| {
        let b = button(name, key, p).label(spoken);
        if on { b.on_press(action) } else { b.disabled(true) }
    };
    row().gap(8.0).center_y().child(step("−", minus, "Menos")).child(text(caption).key(text_key).font(type_::BODY).color(ink).w(190.0)).child(step("+", plus, "Más"))
}

impl State {
    /// `None` follows the system; the choice is saved with the other settings.
    pub(crate) fn set_theme(&mut self, dark: Option<bool>) {
        self.dark = dark;
        self.change_settings(|s| s.dark = dark);
    }
    /// Re-reads the MCP heartbeats; true when an agent connected or disconnected.
    pub(crate) fn check_agents(&mut self) -> bool {
        let Some(home) = &self.home_db else { return false };
        let dir = agents::heartbeat_dir(home);
        let now = std::time::SystemTime::now();
        let connected = Agent::ALL.map(|a| agents::connected(&dir, a, now));
        std::mem::replace(&mut self.agents_connected, connected) != connected
    }
    /// The server installed beside the app (macOS bundle, Windows folder, /usr/bin, target/…).
    pub(crate) fn mcp_command() -> std::path::PathBuf {
        let name = format!("finanzas-mcp{}", std::env::consts::EXE_SUFFIX);
        std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join(&name))).unwrap_or_else(|| name.into())
    }
}

fn theme_card(s: &State, p: Palette) -> El {
    let mut options = row().wrap().gap(8.0);
    for (label, key, value) in [("Automático", "tema-auto", None), ("Claro", "tema-claro", Some(false)), ("Oscuro", "tema-oscuro", Some(true))] {
        let active = s.dark == value;
        let b = button(label, key, p).on_press(move |s: &mut State| s.set_theme(value));
        options = options.child(if active { b.border(2.0, p.petroleum) } else { b });
    }
    card(p).child(icon_title(&theme::GEAR, "Tema", p.petroleum, p))
        .child(muted("Automático sigue el modo claro u oscuro del sistema.", p).wrap().w_full())
        .child(options)
}

fn agents_card(s: &State, p: Palette) -> El {
    let command = State::mcp_command();
    let mut c = card(p).child(icon_title(&theme::DATABASE, "Agentes MCP", p.emerald, p))
        .child(muted("Conecta un agente de IA para consultar y registrar tus finanzas. El indicador se pone verde mientras el agente tiene Finanzas abierto.", p).wrap().w_full());
    for (i, agent) in Agent::ALL.into_iter().enumerate() {
        let on = s.agents_connected[i];
        let mut block = col().w_full().gap(8.0).key(format!("agente-{}", agent.name()))
            .child(row().gap(10.0).center_y()
                .child(el().size(10.0, 10.0).radius(5.0).fill(if on { p.emerald } else { p.border }).label(if on { "Conectado" } else { "Sin conexión" }))
                .child(text(agent.name()).font(type_::TITLE).color(p.ink))
                .child(muted(if on { "Conectado" } else { "Sin conexión" }, p)));
        if !on {
            let config = agent.config(&command);
            let copied = s.agent_copied == Some(i);
            block = block.child(muted(agent.instructions(), p).wrap().w_full())
                .child(el().w_full().padding(12.0).radius(10.0).fill(p.canvas).border(1.0, p.border).child(text(config.clone()).font(type_::BODY).color(p.ink).wrap_anywhere().w_full()))
                .child(row().gap(10.0).center_y()
                    .child(button("Copiar configuración", &format!("agente-copiar-{i}"), p).on_press_tx(move |s: &mut State, tx: &mut Tx| { tx.copy(config.clone()); s.agent_copied = Some(i); }))
                    .child(if copied { muted("Copiado", p) } else { el() }));
        }
        c = c.child(block);
    }
    c
}

pub fn render(s: &State, p: Palette) -> El {
    let on = s.settings.enabled;
    let busy = s.mutation_active();
    let import_button = |name: &'static str, key: &'static str, kind: data::ImportKind| {
        let b = button(name, key, p);
        if busy { b.disabled(true) } else { b.on_press_tx(move |s: &mut State, tx: &mut Tx| choose(s, tx, kind)) }
    };
    let mut reminders = card(p).child(icon_title(&theme::BELL, "Recordatorios", p.amber, p))
        .child(toggle("Avisarme antes de cada pago", "reminders-toggle", on, p).on_press(|s: &mut State| s.change_settings(|v| v.enabled = !v.enabled)))
        .child(stepper("days-value", format!("Avisar {} día(s) antes", s.settings.days), ("days-minus", |s| s.change_settings(|v| v.days = (v.days - 1).max(0))), ("days-plus", |s| s.change_settings(|v| v.days = (v.days + 1).min(settings::MAX_DAYS))), on, p))
        .child(stepper("hour-value", format!("a las {}:00", s.settings.hour), ("hour-minus", |s| s.change_settings(|v| v.hour = v.hour.saturating_sub(1))), ("hour-plus", |s| s.change_settings(|v| v.hour = (v.hour + 1).min(settings::MAX_HOUR))), on, p))
        .child(muted("Los avisos aparecen en Resumen mientras la app está abierta. Los pagos marcados como \"Registrar automáticamente\" se registran solos en su fecha al abrir la app y cada hora.", p).wrap().w_full());
    if let Some(error) = &s.settings_error { reminders = reminders.child(text(error.clone()).key("settings-error").font(type_::BODY).color(p.coral)); }
    let mut import = card(p).child(icon_title(&theme::DOWNLOAD, "Importar datos", p.petroleum, p))
        .child(muted("Copia tus datos a una base de datos vacía. El archivo de origen solo se lee, nunca se modifica.", p).wrap().w_full())
        .child(row().gap(10.0)
            .child(import_button("Importar desde Finanzas (SwiftData)", "import-swiftdata", data::ImportKind::SwiftData))
            .child(import_button("Importar desde Actual", "import-actual", data::ImportKind::Actual)));
    if s.importing { import = import.child(muted("Importando… no cierres la app.", p).key("import-progress")); }
    match &s.import_outcome {
        Some(Ok(summary)) => import = import.child(text(summary_text(summary)).key("import-result").font(type_::BODY).color(p.emerald).wrap().w_full()),
        Some(Err(error)) => import = import.child(text(error.clone()).key("import-error").font(type_::BODY).color(p.coral).wrap().w_full()),
        None => {}
    }
    let status = match &s.update_available { Some(v) => format!("{} · nueva versión {v} disponible", update::CURRENT), None => update::CURRENT.to_owned() };
    let version = card(p).child(icon_title(&theme::DOWNLOAD, "Versión", p.petroleum, p)).child(muted(status, p).key("version-status"))
        .child(muted("Finanzas busca actualizaciones al abrir y una vez al día, y te avisa en Resumen.", p).wrap().w_full());
    let database = card(p).child(icon_title(&theme::DATABASE, "Base de datos", p.emerald, p)).child(field("Ubicación", muted(s.db_label.clone(), p), p));
    col().grow(1.0).h_full().min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(
        col().wrap().w_full().padding(24.0).gap(16.0)
            .child(header_bar(p).child(badge(&theme::GEAR, 16.0, p.petroleum)).child(titles("Tus recordatorios, a tu ritmo", headline("Ajustes", p), p)))
            .child(reminders).child(import).child(perfiles::card_view(s, p)).child(theme_card(s, p)).child(agents_card(s, p)).child(database).child(version))
}

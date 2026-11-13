//! Profiles: `personal` is the default database, every other profile is `<slug>.db` beside it.
//! Creating opens the new file; rename and delete move or remove files, never `personal`.
use super::*;
use finanzas_core::store::Store;
use overview::{card, muted};
use rune::text_input;
use sheet::button;
use theme::icon_title;

impl State {
    /// Reload everything from another profile's database, keeping the theme, profile list and wakers.
    pub(crate) fn switch_to(&mut self, profile: String) {
        if self.profile == profile { return; }
        let Some(home) = self.home_db.clone() else { return };
        let (dark, logo, waker, profiles) = (self.dark, self.logo.take(), self.waker.take(), std::mem::take(&mut self.profiles));
        *self = State::load_from(Store::profile_path_beside(&home, &profile));
        self.home_db = Some(home);
        self.profile = profile; self.dark = dark; self.logo = logo; self.profiles = profiles;
        if let Some(waker) = waker { setup(self, waker); }
    }
    pub(crate) fn visible_profiles(&self) -> Vec<String> {
        self.profiles.iter().filter(|p| **p == self.profile || !self.settings.hidden_profiles.contains(p)).cloned().collect()
    }
    fn refresh_profiles(&mut self) {
        if let Some(home) = &self.home_db { self.profiles = Store::profiles_beside(home); }
    }
    pub(crate) fn start_profile_edit(&mut self, from: Option<String>) {
        let text = from.as_deref().map(Store::profile_label).unwrap_or_default();
        self.profile_edit = Some((from, TextEdit::new(&text)));
        self.profile_error = None;
        self.route = Route::Ajustes;
    }
    /// Creates (and switches to) a new profile, or renames one; the active profile is closed first.
    pub(crate) fn submit_profile_edit(&mut self) {
        let Some((from, edit)) = &self.profile_edit else { return };
        let Some(home) = self.home_db.clone() else { return };
        let Some(slug) = Store::profile_slug(&edit.text) else { self.profile_error = Some("Escribe un nombre con letras o números".into()); return };
        let from = from.clone();
        if from.as_deref() == Some(slug.as_str()) { self.profile_edit = None; return; }
        if self.profiles.contains(&slug) { self.profile_error = Some("Ya existe un perfil con ese nombre".into()); return; }
        match from {
            None => {
                self.profile_edit = None;
                self.profiles.push(slug.clone());
                self.switch_to(slug);
            }
            Some(from) => {
                let was_active = self.profile == from;
                if was_active { self.switch_to("personal".into()); }
                if let Err(e) = Store::rename_profile(&home, &from, &slug) {
                    if was_active { self.switch_to(from); }
                    self.route = Route::Ajustes;
                    self.profile_error = Some(e);
                    return;
                }
                self.profile_edit = None; self.profile_error = None;
                if self.settings.hidden_profiles.contains(&from) {
                    self.change_settings(|s| for p in s.hidden_profiles.iter_mut() { if *p == from { *p = slug.clone(); } });
                }
                self.refresh_profiles();
                if was_active { self.switch_to(slug); }
                self.route = Route::Ajustes;
            }
        }
    }
    pub(crate) fn toggle_profile_hidden(&mut self, profile: &str) {
        if profile == "personal" { return; }
        let profile = profile.to_owned();
        self.change_settings(|s| if let Some(i) = s.hidden_profiles.iter().position(|p| *p == profile) { s.hidden_profiles.remove(i); } else { s.hidden_profiles.push(profile); });
    }
    pub(crate) fn request_profile_delete(&mut self, profile: &str) {
        if profile != "personal" && self.profiles.iter().any(|p| p == profile) { self.profile_delete = Some(profile.to_owned()); self.profile_error = None; }
    }
    /// Deletes the confirmed profile's files; the active profile is closed (switched to personal) first.
    pub(crate) fn confirm_profile_delete(&mut self) {
        let (Some(profile), Some(home)) = (self.profile_delete.take(), self.home_db.clone()) else { return };
        if self.profile == profile { self.switch_to("personal".into()); }
        self.route = Route::Ajustes;
        if let Err(e) = Store::delete_profile(&home, &profile) { self.profile_error = Some(e); return; }
        if self.settings.hidden_profiles.contains(&profile) { self.change_settings(|s| s.hidden_profiles.retain(|p| *p != profile)); }
        self.refresh_profiles();
    }
}

/// Sidebar: the visible profiles and a shortcut to create one.
pub fn sidebar(s: &State, p: Palette) -> El {
    let mut list = col().w_full().gap(2.0);
    for profile in s.visible_profiles() {
        let active = profile == s.profile;
        let label = Store::profile_label(&profile);
        let target = profile.clone();
        list = list.child(row().w_full().h(32.0).px(10.0).center_y().gap(10.0).radius(10.0).key(format!("perfil-{profile}")).role("button").label(label.clone()).focusable(true)
            .fill(if active { p.selected } else { Color::TRANSPARENT }).hover(p.selected)
            .on_press(move |s: &mut State| s.switch_to(target.clone()))
            .child(el().size(8.0, 8.0).radius(4.0).fill(if active { p.petroleum } else { p.border }))
            .child(text(label).font(if active { type_::TITLE } else { type_::BODY }).color(if active { p.petroleum } else { p.ink }).grow(1.0).min_w(0.0).truncate()));
    }
    list.child(row().w_full().h(32.0).px(10.0).center_y().gap(10.0).key("perfil-nuevo").role("button").label("Nuevo perfil").focusable(true).radius(10.0).hover(p.selected)
        .on_press(|s: &mut State| s.start_profile_edit(None))
        .child(icon(&theme::PLUS, 14.0).color(p.petroleum)).child(text("Nuevo perfil").font(type_::BODY).color(p.petroleum)))
}

fn editor(s: &State, p: Palette) -> El {
    let (from, edit) = s.profile_edit.as_ref().expect("profile editor open");
    let action = if from.is_some() { "Renombrar" } else { "Crear" };
    row().w_full().gap(8.0).center_y()
        .child(row().grow(1.0).px(12.0).radius(10.0).center_y().fill(p.canvas).border(1.0, p.border)
            .child(text_input(edit, type_::BODY).key("perfil-nombre").single_line().autofocus()
                .on_edit(|s: &mut State| &mut s.profile_edit.as_mut().expect("profile editor open").1)
                .on_submit(|s: &mut State, _| s.submit_profile_edit())
                .placeholder("Nombre del perfil", p.muted).grow(1.0).py(9.0)))
        .child(button("Cancelar", "perfil-cancelar", p).on_press(|s: &mut State| { s.profile_edit = None; s.profile_error = None; }))
        .child(button(action, "perfil-guardar", p).fill(p.petroleum).default_button().on_press(|s: &mut State| s.submit_profile_edit()))
}

/// Ajustes card: every profile with rename, hide/show and delete; `personal` is fixed.
pub fn card_view(s: &State, p: Palette) -> El {
    let mut c = card(p).child(icon_title(&theme::DATABASE, "Perfiles", p.petroleum, p))
        .child(muted("Cada perfil guarda sus cuentas, presupuesto y pagos por separado. Ocultar solo lo quita de la barra lateral.", p).wrap().w_full());
    for profile in &s.profiles {
        let hidden = s.settings.hidden_profiles.contains(profile);
        let mut name = row().grow(1.0).min_w(0.0).gap(8.0).center_y()
            .child(text(Store::profile_label(profile)).font(type_::TITLE).color(p.ink).truncate());
        if *profile == s.profile { name = name.child(muted("En uso".to_owned(), p)); }
        if hidden { name = name.child(muted("Oculto".to_owned(), p)); }
        let mut line = row().w_full().gap(8.0).center_y().key(format!("perfil-fila-{profile}")).child(name);
        if profile == "personal" {
            line = line.child(muted("Principal".to_owned(), p));
        } else {
            let (a, b, c2) = (profile.clone(), profile.clone(), profile.clone());
            line = line
                .child(button("Renombrar", &format!("perfil-renombrar-{profile}"), p).on_press(move |s: &mut State| s.start_profile_edit(Some(a.clone()))))
                .child(button(if hidden { "Mostrar" } else { "Ocultar" }, &format!("perfil-ocultar-{profile}"), p).on_press(move |s: &mut State| s.toggle_profile_hidden(&b)))
                .child(button("Borrar", &format!("perfil-borrar-{profile}"), p).on_press(move |s: &mut State| s.request_profile_delete(&c2)));
        }
        c = c.child(line);
    }
    c = if s.profile_edit.is_some() { c.child(editor(s, p)) } else {
        c.child(row().child(button("Nuevo perfil", "perfil-crear", p).on_press(|s: &mut State| s.start_profile_edit(None))))
    };
    if let Some(error) = &s.profile_error { c = c.child(text(error.clone()).key("perfil-error").font(type_::BODY).color(p.coral).wrap().w_full()); }
    c
}

pub fn confirm(s: &State, p: Palette) -> El {
    let profile = s.profile_delete.as_ref().expect("profile delete confirmation present");
    let content = col().w(480.0).padding(24.0).gap(14.0).radius(18.0).fill(p.card)
        .child(text(format!("¿Borrar el perfil {}?", Store::profile_label(profile))).font(type_::TITLE).color(p.ink))
        .child(text("Se borran para siempre sus cuentas, movimientos, presupuesto y pagos programados. No se puede deshacer.").font(type_::BODY).color(p.muted).wrap().w_full())
        .child(row().wrap().w_full().justify_between().child(button("Cancelar", "perfil-borrar-cancelar", p).on_press(|s: &mut State| s.profile_delete = None))
            .child(button("Borrar perfil", "perfil-borrar-confirmar", p).fill(p.coral).on_press(|s: &mut State| s.confirm_profile_delete())));
    el().key("confirm-perfil").paint_order(100).raise().absolute(0.0, 0.0).wrap().w_full().h_full().center().fill(Color::BLACK.alpha(0.4)).modal(false, |s: &mut State| s.profile_delete = None).child(content)
}

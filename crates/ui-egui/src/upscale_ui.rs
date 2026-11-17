//! Fotry (Arch Quilly): the Image › Upscale… form, the method and the scale. OK runs `image.upscale`
//! with the values through the generic engine-command family (`filter_dialog::run`).

use craft::ui::contract::Bind;
use craft::ui::dialog::DialogId;
use craft::ui::patterns::form_dialogs::{DialogFields, Param, ParamDialog, ParamKind};
use craft::ui::runtime::env::Label;
use serde_json::Value;

use crate::PhotocraftApp;

pub const COMMAND: &str = "image.upscale";

fn options(items: &[(&str, &str)]) -> Vec<(String, Label)> {
    items.iter().map(|(id, label)| ((*id).to_owned(), Label::from((*label).to_owned()))).collect()
}

fn rows() -> Vec<Param> {
    let methods = options(&[
        ("aiPhoto", tl!("AI: photo (Real-ESRGAN)")),
        ("aiArt", tl!("AI: art and drawings (Real-ESRGAN)")),
        ("lanczos", tl!("Lanczos (sharp, no AI)")),
        ("preserveDetails", tl!("Preserve Details (no AI)")),
    ]);
    let scales = options(&[("2", "2×"), ("3", "3×"), ("4", "4×")]);
    vec![
        Param::new("method", Label::from(tl!("Method")), ParamKind::choice(methods, Some("aiPhoto".to_owned()))),
        Param::new("scale", Label::from(tl!("Scale")), ParamKind::choice(scales, Some("2".to_owned()))),
    ]
}

/// Opens the Upscale form.
pub fn open(app: &mut PhotocraftApp) -> DialogId {
    let params = rows();
    crate::dialogs::params::open_param(app, COMMAND, DialogFields::new(), Value::Null, move |fields| {
        ParamDialog::command(COMMAND, tl!("Upscale"), params.clone()).values(Bind::bound(fields))
    })
}

//! Fonts
//!
//! These functions sometimes simplify the process of using fonts.

use bevy::{asset::AssetServer, ecs::system::Res, text::FontSource};

pub const TEXT_FONT: &'static str = "fonts/Mozilla_Text/MozillaText-VariableFont_wght.ttf";
pub const TITLE_FONT: &'static str =
    "fonts/Mozilla_Headline/MozillaHeadline-VariableFont_wdth,wght.ttf";

pub fn handle(font: &'static str, asset_server: &Res<AssetServer>) -> FontSource {
    FontSource::Handle(asset_server.load(font))
}

pub fn title_font_handle(asset_server: &Res<AssetServer>) -> FontSource {
    handle(TITLE_FONT, asset_server)
}

pub fn text_font_handle(asset_server: &Res<AssetServer>) -> FontSource {
    handle(TEXT_FONT, asset_server)
}

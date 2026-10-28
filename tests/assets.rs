use gpui_kit::AssetSource;
use gpui_kit::component::{IconName, IconNamed};

#[test]
fn app_assets_load_every_icon_the_ui_uses() {
    let assets = gpui_calendar::assets();
    for name in [IconName::Check, IconName::ChevronLeft, IconName::ChevronRight, IconName::ChevronsUpDown, IconName::Plus] {
        let path = name.path();
        let svg = assets.load(&path).expect("load").unwrap_or_else(|| panic!("no asset at {path}"));
        assert!(!svg.is_empty(), "{path} is empty");
    }
}

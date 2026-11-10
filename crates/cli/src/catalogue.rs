pub(crate) struct Recipe {
    pub(crate) name: &'static str,
    pub(crate) module: &'static str,
    pub(crate) exports: &'static [&'static str],
    pub(crate) dependencies: &'static [&'static str],
    pub(crate) source: &'static str,
    pub(crate) assets: &'static [(&'static str, &'static str)],
}

macro_rules! recipe {
    ($name:literal, $module:ident, [$($export:ident),+], [$($dependency:literal),*] $(, $asset:expr)?) => {
        Recipe {
            name: $name,
            module: stringify!($module),
            exports: &[$(stringify!($export)),+],
            dependencies: &[$($dependency),*],
            source: include_str!(concat!("../../base/examples/recipes/", stringify!($module), ".rs")),
            assets: &[$($asset)?],
        }
    };
}

// Names, exports, source dependencies and assets have one owner. Source itself
// stays in the examples; rebuilding the CLI embeds those exact current files.
pub(crate) const RECIPES: &[Recipe] = &[
    recipe!("alert", alert, [Alert], []),
    recipe!("aspect-ratio", aspect_ratio, [AspectRatio], []),
    recipe!("attachment", attachment, [Attachment], []),
    recipe!(
        "avatar",
        avatar,
        [Avatar],
        [],
        (
            "assets/preset-avatar.svg",
            include_str!("../../base/examples/assets/preset-avatar.svg")
        )
    ),
    recipe!("badge", badge, [Badge], []),
    recipe!("bubble", bubble, [Bubble, BubbleSide], []),
    recipe!("button-group", button_group, [ButtonGroup], []),
    recipe!("card", card, [Card], []),
    recipe!("chart", chart, [ChartKind, ChartRecipe], []),
    recipe!("code-block", code_block, [CodeBlock], []),
    recipe!(
        "data-table",
        data_table,
        [DataTable, DataTableColumn, DataTableRow, DataTableState],
        ["empty", "input-group", "pagination"]
    ),
    recipe!("empty", empty, [EmptyState], []),
    recipe!("input-group", input_group, [InputGroup], []),
    recipe!("item", item, [Item], []),
    recipe!("kbd", kbd, [Kbd, KbdGroup], []),
    recipe!("marker", marker, [Marker], []),
    recipe!("message", message, [Message], ["typography"]),
    recipe!("pagination", pagination, [Pagination], ["button-group"]),
    recipe!("sheet", sheet, [Sheet, SheetSide], []),
    recipe!("sidebar", sidebar, [Sidebar], []),
    recipe!("skeleton", skeleton, [Skeleton], []),
    recipe!("spinner", spinner, [Spinner], []),
    recipe!("trace", trace, [Trace], ["typography"]),
    recipe!(
        "typography",
        typography,
        [Heading, InlineCode, Paragraph],
        []
    ),
];

pub(crate) const PRESENTATION: &str = include_str!("../../base/examples/presentation.rs");
pub(crate) const LICENSE: &str = include_str!("../../../LICENSE-APACHE");

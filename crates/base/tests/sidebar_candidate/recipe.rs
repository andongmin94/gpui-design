// Load the exact candidate without registering it in examples/recipes/mod.rs.
// Sidebar's super::super::Palette resolves to this test target's fixture.
#[path = "../../examples/recipes/sidebar.rs"]
pub(crate) mod sidebar;

//! Native compositions for navigation, notifications, media/data, and conversation.
//! Run `cargo run -p gpui-design --example catalogue --locked`.

mod catalogue_conversation;
mod catalogue_data;
mod catalogue_forms;
mod catalogue_navigation;
mod catalogue_selection;

use catalogue_conversation::ConversationCatalogue;
use catalogue_data::DataCatalogue;
use catalogue_forms::CatalogueForms;
use catalogue_navigation::NavigationCatalogue;
use catalogue_selection::SelectionCatalogue;
use gpui::{
    AppContext as _, Context, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, Styled as _, Subscription, Window, div, px, rgb,
};
use gpui_design::{ScrollArea, Tab, TabItem, TabList, TabPanel, TabsEvent, TabsState};

pub(crate) struct Catalogue {
    pub(crate) tabs: Entity<TabsState>,
    navigation: Entity<NavigationCatalogue>,
    forms: Entity<CatalogueForms>,
    data: Entity<DataCatalogue>,
    conversation: Entity<ConversationCatalogue>,
    selection: Entity<SelectionCatalogue>,
    scroll: ScrollHandle,
    _selection: Subscription,
    _tabs_observation: Subscription,
}

impl Catalogue {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tabs = cx.new(|cx| {
            TabsState::new(
                [
                    TabItem::new("navigation", "Navigation"),
                    TabItem::new("forms", "Forms and feedback"),
                    TabItem::new("data", "Media and data"),
                    TabItem::new("conversation", "Conversation"),
                    TabItem::new("selection", "Selection"),
                ],
                cx,
            )
            .manual_activation()
        });
        let selection = cx.subscribe(&tabs, |this, _, event, cx| {
            let TabsEvent::Change(_) = event;
            this.scroll.set_offset(gpui::point(px(0.), px(0.)));
            cx.notify();
        });
        let tabs_observation = cx.observe(&tabs, |_, _, cx| cx.notify());
        Self {
            tabs,
            navigation: cx.new(|cx| NavigationCatalogue::new(window, cx)),
            forms: cx.new(|cx| CatalogueForms::new(window, cx)),
            data: cx.new(|cx| DataCatalogue::new(window, cx)),
            conversation: cx.new(|cx| ConversationCatalogue::new(window, cx)),
            selection: cx.new(|cx| SelectionCatalogue::new(window, cx)),
            scroll: ScrollHandle::new(),
            _selection: selection,
            _tabs_observation: tabs_observation,
        }
    }
}

impl Render for Catalogue {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = self.tabs.read(cx);
        let entries = tabs
            .items()
            .iter()
            .enumerate()
            .map(|(index, item)| {
                Tab::new(&self.tabs, index)
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(if tabs.selected_index() == Some(index) {
                        0x1d4ed8
                    } else {
                        0x172033
                    }))
                    .hover(|style| style.bg(rgb(0x334155)))
                    .focus_visible(|style| style.bg(rgb(0x93c5fd)).text_color(rgb(0x0b1220)))
                    .child(item.label().clone())
            })
            .collect::<Vec<_>>();
        div().size_full().flex().flex_col().gap_4().p_5()
            .bg(rgb(0x0b1220)).text_color(rgb(0xe2e8f0))
            .child(div().text_2xl().font_weight(FontWeight::BOLD).child("GPUI Design · Catalogue"))
            .child(div().text_sm().text_color(rgb(0x94a3b8)).child("Native behavior. Application-owned presentation. Explore each working composition."))
            .child(TabList::new("catalogue-sections",&self.tabs).accessibility_label("Component catalogue")
                .flex().flex_wrap().gap_2().children(entries))
            .child(div().text_sm().text_color(rgb(0x94a3b8))
                .child("Use Left/Right to focus a section, then Enter or Space to open it. You can also click a section."))
            .child(ScrollArea::new("catalogue-content",&self.scroll)
                .accessibility_label("Catalogue content").flex_1().min_h_0().w_full()
                .content_styles(|style| style.flex().flex_col().gap_4())
                .child(TabPanel::new(&self.tabs,0).w_full().child(self.navigation.clone()))
                .child(TabPanel::new(&self.tabs,1).w_full().child(self.forms.clone()))
                .child(TabPanel::new(&self.tabs,2).w_full().child(self.data.clone()))
                .child(TabPanel::new(&self.tabs,3).w_full().child(self.conversation.clone()))
                .child(TabPanel::new(&self.tabs,4).w_full().child(self.selection.clone())))
    }
}

#[cfg(not(test))]
fn main() {
    use gpui::{TitlebarOptions, WindowBounds, WindowOptions, size};
    use gpui_design::Root;
    gpui_platform::application().run(|cx| {
        gpui_design::init(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("GPUI Design · Catalogue".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::centered(size(px(980.), px(840.)), cx)),
                ..WindowOptions::default()
            },
            |window, cx| {
                let catalogue = cx.new(|cx| Catalogue::new(window, cx));
                cx.new(|cx| Root::new(catalogue, window, cx))
            },
        )
        .expect("failed to open catalogue example");
        cx.activate(true);
    });
}

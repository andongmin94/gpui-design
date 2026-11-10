//! Native compositions for navigation, notifications, media/data, and conversation.
//! Run `cargo run -p gpui-design --example catalogue --locked`.

mod catalogue_conversation;
mod catalogue_data;
mod catalogue_forms;
mod catalogue_navigation;

use catalogue_conversation::ConversationCatalogue;
use catalogue_data::DataCatalogue;
use catalogue_forms::CatalogueForms;
use catalogue_navigation::NavigationCatalogue;
use gpui::{
    AppContext as _, Context, Entity, FontWeight, IntoElement, ParentElement as _, Render,
    ScrollHandle, Styled as _, Subscription, Window, div, px, rgb,
};
use gpui_design::{ScrollArea, Tab, TabItem, TabList, TabPanel, TabsState};

pub(crate) struct Catalogue {
    pub(crate) tabs: Entity<TabsState>,
    navigation: Entity<NavigationCatalogue>,
    forms: Entity<CatalogueForms>,
    data: Entity<DataCatalogue>,
    conversation: Entity<ConversationCatalogue>,
    scroll: ScrollHandle,
    _selection: Subscription,
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
                ],
                cx,
            )
        });
        let selection = cx.observe(&tabs, |this, _, cx| {
            this.scroll.set_offset(gpui::point(px(0.), px(0.)));
            cx.notify();
        });
        Self {
            tabs,
            navigation: cx.new(|cx| NavigationCatalogue::new(window, cx)),
            forms: cx.new(|cx| CatalogueForms::new(window, cx)),
            data: cx.new(|cx| DataCatalogue::new(window, cx)),
            conversation: cx.new(|cx| ConversationCatalogue::new(window, cx)),
            scroll: ScrollHandle::new(),
            _selection: selection,
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
                    .child(item.label().clone())
            })
            .collect::<Vec<_>>();
        div().size_full().flex().flex_col().gap_4().p_5()
            .bg(rgb(0x0b1220)).text_color(rgb(0xe2e8f0))
            .child(div().text_2xl().font_weight(FontWeight::BOLD).child("GPUI Design · Catalogue"))
            .child(div().text_sm().text_color(rgb(0x94a3b8)).child("Native behavior. Application-owned presentation. Explore each working composition."))
            .child(TabList::new("catalogue-sections",&self.tabs).accessibility_label("Component catalogue")
                .flex().flex_wrap().gap_2().children(entries))
            .child(ScrollArea::new("catalogue-content",&self.scroll)
                .accessibility_label("Catalogue content").flex_1().min_h_0().w_full()
                .content_styles(|style| style.flex().flex_col().gap_4())
                .child(TabPanel::new(&self.tabs,0).w_full().child(self.navigation.clone()))
                .child(TabPanel::new(&self.tabs,1).w_full().child(self.forms.clone()))
                .child(TabPanel::new(&self.tabs,2).w_full().child(self.data.clone()))
                .child(TabPanel::new(&self.tabs,3).w_full().child(self.conversation.clone())))
    }
}

#[cfg(not(test))]
fn main() {
    use gpui::{WindowBounds, WindowOptions, size};
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

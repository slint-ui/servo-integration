use std::{cell::RefCell, rc::Rc};

use compositing::IOCompositor;
use servo::{FocusId, ScreenGeometry, WebViewId};

use compositing_traits::WebViewTrait;
use constellation_traits::EmbedderToConstellationMessage;

use super::constellation::ConstellationProxy;

#[derive(Clone)]
pub struct WebView {
    pub id: WebViewId,
    pub compositor: Rc<RefCell<IOCompositor>>,
    pub constellation_proxy: ConstellationProxy,
}

impl WebViewTrait for WebView {
    fn id(&self) -> WebViewId {
        self.id
    }

    fn screen_geometry(&self) -> Option<ScreenGeometry> {
        todo!()
    }

    fn set_animating(&self, _: bool) {}
}

impl WebView {
    /// Paint the contents of this [`WebView`] into its `RenderingContext`. This will
    /// always paint, unless the `Opts::wait_for_stable_image` option is enabled. In
    /// that case, this might do nothing. Returns true if a paint was actually performed.
    pub fn paint(&self) -> bool {
        self.compositor.borrow_mut().render()
    }

    pub fn focus(&self) {
        let focus_id = FocusId::new();
        self.constellation_proxy
            .send(EmbedderToConstellationMessage::FocusWebView(
                self.id(),
                focus_id.clone(),
            ));
    }

    pub fn raise_to_top(&self, hide_others: bool) {
        self.compositor
            .borrow_mut()
            .raise_webview_to_top(self.id(), hide_others)
            .expect("BUG: invalid WebView instance");
    }
}

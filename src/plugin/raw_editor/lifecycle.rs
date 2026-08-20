use std::sync::Arc;

use baseview::{Window, WindowOpenOptions, WindowScalePolicy};
use truce::core::editor::RawWindowHandle;
use truce::prelude::{Editor, PluginContext};
use truce_gui::platform::{ParentWindow, query_backing_scale};

use crate::plugin::params::PluginParams;

use super::{SIZE, interaction::Handler, renderer::Renderer};

pub(in crate::plugin) struct RawEditor {
    params: Arc<PluginParams>,
    window: Option<baseview::WindowHandle>,
}

#[expect(
    unsafe_code,
    clippy::non_send_fields_in_send_ty,
    reason = "Truce requires Editor: Send while baseview deliberately marks WindowHandle !Send; the host keeps every editor method on its UI thread"
)]
// SAFETY: Truce requires `Editor: Send`, but the framework invokes every editor
// method on the UI thread, where the baseview handle remains confined.
unsafe impl Send for RawEditor {}

impl RawEditor {
    pub(in crate::plugin) const fn new(params: Arc<PluginParams>) -> Self {
        Self {
            params,
            window: None,
        }
    }
}

impl Editor for RawEditor {
    fn size(&self) -> (u32, u32) {
        SIZE
    }

    #[expect(
        unsafe_code,
        reason = "wgpu surface creation is tied to the live baseview child window"
    )]
    fn open(&mut self, parent: RawWindowHandle, context: PluginContext) {
        let plugin_context = context.with_params(Arc::clone(&self.params));
        let scale = query_backing_scale(&parent);
        let physical_size = (
            truce_gui::to_physical_px(SIZE.0, scale),
            truce_gui::to_physical_px(SIZE.1, scale),
        );
        let params = Arc::clone(&self.params);
        let options = WindowOpenOptions {
            title: String::from("Delay Lama"),
            size: baseview::Size::new(f64::from(SIZE.0), f64::from(SIZE.1)),
            scale: WindowScalePolicy::SystemScaleFactor,
        };
        self.window = Some(Window::open_parented(
            &ParentWindow(parent),
            options,
            move |window| {
                Handler::new(
                    // SAFETY: The child window outlives the renderer created for this callback.
                    unsafe { Renderer::new(window, physical_size.0, physical_size.1) },
                    params,
                    plugin_context,
                    SIZE,
                )
            },
        ));
    }

    fn close(&mut self) {
        if let Some(mut window) = self.window.take() {
            window.close();
        }
    }

    fn can_resize(&self) -> bool {
        false
    }
}

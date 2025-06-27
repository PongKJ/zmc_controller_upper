use leptos::{prelude::*, server::codee::string::JsonSerdeCodec};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Outlet, ParentRoute, Route, Router, Routes},
    StaticSegment,
};
use leptos_use::use_cookie;
use thaw::ssr::SSRMountStyleProvider;
use thaw::*;

use crate::components::*;

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GlobalState {
    pub connected: bool,
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <SSRMountStyleProvider>
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <AutoReload options=options.clone() />
                    <HydrationScripts options />
                    <MetaTags />
                </head>
                <body>
                    <App />
                </body>
            </html>
        </SSRMountStyleProvider>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    // Provides context for WebSocket connections
    leptos_ws::provide_websocket("ws://localhost:3000/ws");


    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos_ssr_startup.css" />

        <Title text="Welcome to Leptos" />

        <ConfigProvider>
            <ToasterProvider>
                <Router>
                    <main>
                        <Routes fallback=|| "Page not found.".into_view()>
                            <ParentRoute path=StaticSegment("") view=HomePage>
                                <Route path=StaticSegment("parameters") view=ParametersView />
                                <Route path=StaticSegment("manual") view=ManualView />
                                <Route path=StaticSegment("about") view=AboutView />
                                <Route path=StaticSegment("auto") view=AutoModeView />
                            </ParentRoute>
                        </Routes>
                    </main>
                </Router>

            </ToasterProvider>
        </ConfigProvider>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let (global_state, set_global_state) =
        use_cookie::<GlobalState, JsonSerdeCodec>("global_state_cookie");
    // Ensure global state is initialized
    if global_state.read_untracked().is_none() {
        set_global_state.set(Some(GlobalState::default()));
    }

    let connected = move || global_state.get().unwrap().connected;

    view! {
        <Flex>
            <Flex align=FlexAlign::Start class="flex-left">
                <NavDrawer>
                    <NavItem value="parameters" href="/parameters">
                        "Parameters"
                    </NavItem>
                    <NavItem value="manual" href="/manual">
                        "Manual Control"
                    </NavItem>
                    <NavItem value="auto" href="/auto">
                        "Auto mode"
                    </NavItem>
                    <NavItem value="about" href="about">
                        "About"
                    </NavItem>
                    <NavDrawerFooter slot>
                        <LimitStatusView />
                        <Badge color=Signal::derive(move || {
                            if connected() { BadgeColor::Success } else { BadgeColor::Severe }
                        })>
                            {move || { if connected() { "Connected" } else { "Disconnected" } }}
                        </Badge>
                    </NavDrawerFooter>
                </NavDrawer>
            </Flex>
            <Flex align=FlexAlign::Center class="flex-center">
                <Outlet />
            </Flex>
            <Flex align=FlexAlign::End class="flex-right">
                <VisualView />
            </Flex>
        </Flex>
    }
}

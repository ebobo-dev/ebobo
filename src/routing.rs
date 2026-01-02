use sycamore::prelude::*;
use sycamore_router::{HistoryIntegration, Route, Router};

use crate::components;

#[component(inline_props)]
pub async fn Root() -> View {
    view! {
        Router(
            integration=HistoryIntegration::new(),
            view=|target: ReadSignal<AppRoutes>| view! {
                (match target.get() {
                AppRoutes::Index => view! { components::index::Index() },
                AppRoutes::NotFound => view! { "lost?"}})
            }
        )
    }
}

#[derive(Route, Clone, Copy, Debug)]
enum AppRoutes {
    #[to("")]
    Index,
    #[not_found]
    NotFound,
}

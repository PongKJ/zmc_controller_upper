use leptos::prelude::*;
use chrono::Datelike;

#[component]
pub fn AboutView() -> impl IntoView {
    view! {
        <div class="about-container" style="max-width: 600px; margin: 40px auto; padding: 32px; background: #f8fafc; border-radius: 16px; box-shadow: 0 4px 24px rgba(0,0,0,0.08); text-align: center;">
            <h1 style="font-size: 2.0rem; color: #1e293b; margin-bottom: 16px;">Zmc Controller Upper</h1>
            <p style="font-size: 1.2rem; color: #334155; margin-bottom: 24px;">
                <strong>Powered by <span style="color:#ea580c;">Rust</span> + <span style="color:#38bdf8;">Leptos</span></strong>.
            </p>
            <p style="font-size: 1.1rem; color: #64748b; margin-bottom: 32px;">
                Made by <a href="https://your-profile-link" style="color:#2563eb; text-decoration:underline;">Group B12</a>
            </p>
            <div style="font-size: 0.95rem; color: #94a3b8;">
                copy; {chrono::Utc::now().year()} All rights reserved.
            </div>
        </div>
    }
}


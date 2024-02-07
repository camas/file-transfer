use git_version::git_version;
use leptos::*;

#[component]
pub(crate) fn Footer() -> impl IntoView {
    let git_commit_hash = git_version!(args = ["--always", "--abbrev=40"]);
    let github_commit_url =
        format!("https://github.com/camas/file-transfer/commit/{git_commit_hash}");
    let git_version_string = git_version!(args = ["--always", "--abbrev=8", "--dirty=-dirty"]);

    view! {
        <div class="footer">
            <div>"file-transfer-"<a href=github_commit_url target="_blank" rel="noopener">{git_version_string}</a></div>
            <div></div>
            <div>"View on "<a href="https://github.com/camas/file-transfer" target="_blank" rel="noopener">"GitHub"</a></div>
        </div>
    }
}

# Register GitHub authentication under do-me

The editor source is [do-me/eo-glossary-editor](https://github.com/do-me/eo-glossary-editor). The default glossary data repository remains [ceos-org/eo-glossary](https://github.com/ceos-org/eo-glossary); the test data fork is [do-me/eo-glossary](https://github.com/do-me/eo-glossary). App source ownership, GitHub App registration ownership, and repositories the app edits are separate settings.

## 1. Create the registration

Sign into GitHub as **do-me**, then open [New GitHub App](https://github.com/settings/apps/new). This is a personal-account registration and does not require CEOS organization-owner access.

| GitHub setting | Value |
|---|---|
| GitHub App name | EO Glossary Editor by do-me (or another globally available name) |
| Description | Desktop editor for reviewing and contributing CEOS Earth observation glossary terms. |
| Homepage URL | `https://github.com/do-me/eo-glossary-editor` |
| Callback URL | Empty; device authorization does not use a callback |
| Expire user authorization tokens | Checked |
| Request user authorization (OAuth) during installation | Unchecked |
| Enable Device Flow | Checked |
| Setup URL | Empty |
| Webhook → Active | Unchecked |
| Repository permissions → Contents | Read and write |
| Repository permissions → Pull requests | Read and write |
| Repository permissions → Metadata | Read only (automatic) |
| Other repository, organization, and account permissions | No access |
| Where can this GitHub App be installed? | Any account |

Click **Create GitHub App**. This registration is free and needs no hosted service, callback server, app private key, or client secret for the implemented device flow. See [GitHub registration instructions](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/registering-a-github-app).

## 2. Install on the glossary test fork

On the new app's settings page, choose **Install App → Install** beside **do-me → Only select repositories → eo-glossary → Install**. The permission grant is to the glossary data repository, not the editor source repository. Later, install on `ceos-org/eo-glossary` for production contributions, subject to CEOS approval/policy. Contributors using other personal forks may also need an installation on those forks. See [GitHub installation instructions](https://docs.github.com/en/apps/using-github-apps/installing-your-own-github-app).

## 3. Configure and sign in from the editor

On the GitHub App's **General** settings page, copy **Client ID**, not the numeric App ID. In the editor open **Settings → Configure GitHub authentication → Application type: GitHub App (recommended) → Public client ID**. Paste it and click **Save client ID**.

The **Authenticate with GitHub** button becomes enabled. Click it, open the displayed GitHub device authorization page, and enter/approve the displayed code. Remembered credentials stay in the OS credential store.

For test commits/PRs, select `https://github.com/do-me/eo-glossary` in the editor's **Settings → Repository URL → Use repository**. This opens a session pinned to that fork. Earlier sessions keep their own destination; inspect the destination on the review page before submitting.

## Distribution

This is one-time maintainer setup. A release can embed the public client ID using `EO_GITHUB_CLIENT_ID` and `EO_GITHUB_AUTH_KIND=github_app` at build time, so experts simply click Authenticate with GitHub. No secret is embedded. No client ID has been configured in the current development build.

For an existing OAuth app, enable Device Flow in its registration and select OAuth app in the editor. Legacy client-ID settings retain OAuth behavior. Classic `public_repo` PAT fallback remains available for testing.

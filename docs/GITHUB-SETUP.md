# GitHub sign-in with a personal access token

Personal access tokens are the primary supported route. A GitHub App registration is unnecessary for normal use.

## Personal token setup

1. Open [GitHub’s classic token creation page](https://github.com/settings/tokens/new) while signed in as the account that will contribute.
2. Enter a name such as **EO Glossary Editor**, choose an expiration, and select only **public_repo** for the public glossary.
3. Generate the token, copy it, then open the editor’s **Settings → Personal access token** field.
4. Paste it and click **Sign in with token**. Keep Remember me checked to use the system credential store, or uncheck it for this launch only.

The default target is `ceos-org/eo-glossary`. Organization policies and SSO approval still apply. A token never grants rights the account itself lacks. Fine-grained tokens may work for users with write access to the target: select the repository owner and glossary repository, with Contents and Pull requests read/write permissions. The cross-owner fork workflow can require a classic token because fine-grained tokens have contributor/owner limitations. See [GitHub’s token documentation](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens).

Tokens are excluded from session exports and source files. If a token expires or is revoked, generate another and sign in again. Never put a token in a glossary term or PR description.

## Optional future experiment: GitHub App authentication

The steps below document retained experimental support. They are not onboarding requirements or a committed roadmap.

### Registration under do-me

The editor source is [do-me/eo-glossary-editor](https://github.com/do-me/eo-glossary-editor). The default glossary data repository remains [ceos-org/eo-glossary](https://github.com/ceos-org/eo-glossary); the test data fork is [do-me/eo-glossary](https://github.com/do-me/eo-glossary). App source ownership, GitHub App registration ownership, and repositories the app edits are separate settings.

### 1. Create the registration

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

### 2. Install on the production glossary

On the new app's settings page, choose **Install App → Install** beside **ceos-org → Only select repositories → eo-glossary → Install**. The permission grant is to `ceos-org/eo-glossary`. The editor source repository is only the homepage. If GitHub requires organization approval, a CEOS owner must approve the installation. See [GitHub installation instructions](https://docs.github.com/en/apps/using-github-apps/installing-your-own-github-app).

With the signed-in user's CEOS write access, the editor creates a session branch in CEOS and opens a PR against main. Contributors without upstream write access use their own fork through the existing contributor workflow.

### 3. Configure and sign in from the editor

On the GitHub App's **General** settings page, copy **Client ID**, not the numeric App ID. In the editor open **Settings → Advanced: optional app sign-in → Application type: GitHub App → Public client ID**. Paste it and click **Save client ID**.

The **Sign in with registered app** button becomes available. Click it, open the displayed GitHub device authorization page, and enter/approve the displayed code. Remembered credentials stay in the OS credential store.

For test commits/PRs, select `https://github.com/do-me/eo-glossary` in the editor's **Settings → Repository URL → Use repository**. This opens a session pinned to that fork. Earlier sessions keep their own destination; inspect the destination on the review page before submitting.

### Optional distribution configuration

This is one-time maintainer setup. A release can embed the public client ID using `EO_GITHUB_CLIENT_ID` and `EO_GITHUB_AUTH_KIND=github_app` at build time, so an optional app-sign-in deployment could avoid manual client-ID entry. No secret is embedded. No client ID has been configured in the current development build.

For an existing OAuth app, enable Device Flow in its registration and select OAuth app in the editor. Legacy client-ID settings retain OAuth behavior. Personal tokens remain the primary workflow regardless of this optional configuration.

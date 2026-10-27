# GitHub App or OAuth app?

Status: historical comparison. The maintainer chose personal access tokens as primary on 27 October 2026; see [Decision 003](003-personal-access-tokens.md). GitHub App deployment is optional future work.

Decision, 26 October 2026: prefer a public **GitHub App using device authorization and user access tokens** for distribution. Retain OAuth-app and PAT compatibility for pilots.

| Concern | GitHub App | OAuth app |
|---|---|---|
| Repository access | Selected repositories and explicit permissions | `public_repo` covers public repositories broadly |
| Expert identity | User tokens attribute actions to the expert | Acts as the authorized expert |
| Local desktop / no hosting | Device flow works directly with GitHub | Device flow works directly with GitHub |
| Token lifecycle | Expiring user token with refresh rotation | Expiring-token support also available |
| Contributor setup | Install on the glossary and, where required, the contributor fork | Simpler authorization, broader scope |
| GitHub plan | Available on Free | No paid plan required for this workflow |

Use only Contents read/write, Pull requests read/write, and automatic Metadata read access. No organization permissions, webhooks, application private key, or embedded client secret are needed for this user-token device flow. Refresh tokens stay in native secure storage. The app supports both client types and avoids sending OAuth `public_repo` scope for GitHub App authentication.

This recommendation prioritizes limited permissions for occasional expert contributors. The tradeoff is installation approval on the target repository and sometimes the personal fork. Verify the ordinary-contributor installation/fork journey before distributing publicly; PAT fallback remains available.

`gh` confirms `do-me` has repository administrator permission for `ceos-org/eo-glossary`, but its CEOS organization membership is `member`, not `admin` (owner). Repository admin can be sufficient to install an existing GitHub App, subject to organization policy. Managing an organization-owned registration normally requires an organization owner or a delegated GitHub App manager. Device Flow is not a repository setting. There is no supported `gh` command/API switch for enabling it on a registration; GitHub documents the browser setting. A personal-account registration is another option.

The required public client ID is still absent. This implementation has not registered an app, changed organization access policy, installed an app, or borrowed GitHub CLI's client identity.

References:

- [GitHub Apps versus OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps).
- [Device authorization for a GitHub App](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app).
- [Refresh tokens without a client secret for device-issued tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens).
- [Enable Device Flow on a GitHub App registration](https://docs.github.com/en/apps/maintaining-github-apps/modifying-a-github-app-registration#enabling-or-disabling-the-device-flow).
- [GitHub pricing: GitHub Apps are included on Free](https://github.com/pricing).

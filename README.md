# hydcraft-updater

## CNB release

GitHub Actions runs CI and mirrors `main` to CNB. Use the GitHub `Deploy updater via CNB` workflow or the CNB `Deploy updater` button to build the Windows and macOS artifacts on native self-hosted runners.

The CNB secret file is `HydCraft/hydcraft-secrets/hydcraft-updater-deploy.yml`. It contains the Tencent COS S3 endpoint, bucket, an object prefix under `updater/`, COS credentials, the Console origin, and a Console publish token with the `UPDATER` scope. Configure `dl-updater` in Console as the public source base URL `https://dl-shanghai-cdn.hydcraft.cn`; EdgeOne exposes only `/updater/*` without authentication, and Console turns each reported object key into the Bootstrap download URL. After both platform callbacks publish a version, CNB removes the archived updater objects returned by Console.

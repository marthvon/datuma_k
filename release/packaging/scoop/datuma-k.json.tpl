{
    "version": "@VERSION@",
    "description": "Data contract plus templates that generate source",
    "homepage": "@REPO_HOMEPAGE@",
    "license": "AGPL-3.0-only",
    "architecture": {
        "64bit": {
            "url": "@BASE_URL@/datuma_k-windows-x86_64.exe#/datuma_k.exe",
            "hash": "@SHA256_WINDOWS_X86_64@"
        }
    },
    "bin": "datuma_k.exe",
    "checkver": {
        "github": "@REPO_HOMEPAGE@"
    },
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/marthvon/datuma_k/releases/download/v$version/datuma_k-windows-x86_64.exe#/datuma_k.exe"
            }
        }
    }
}

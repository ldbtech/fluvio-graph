src/
└── ingestion_registry/
    └── email/
        ├── mod.rs
        │
        ├── auth/
        │   ├── mod.rs
        │   ├── oauth.rs        # OAuth2 flow — get auth URL, exchange code, refresh token
        │   └── token_store.rs  # read/write ~/.fluvio/credentials/gmail.json
        │
        ├── client/
        │   ├── mod.rs
        │   ├── gmail.rs        # Gmail API calls — list messages, get thread, get labels
        │   └── models.rs       # raw Gmail API response structs (deserialize from JSON)
        │
        ├── sync/
        │   ├── mod.rs
        │   ├── full.rs         # full sync — paginate all mail, respect rate limits
        │   ├── incremental.rs  # incremental — historyId based, pull only new/changed
        │   └── state.rs        # SyncState struct — stores historyId, last_sync_at in ~/.fluvio/
        │
        ├── connector.rs        # GmailConnector — implements FluvioConnector trait
        └── normalizer.rs       # raw Gmail API model → NormalizedChunk + metadata
# Affinidi Trust Network - Affinidi Trusted Messaging

## Changelog history

### 15th September 2024 (release 0.1.9)

* clarity: Added a note regarding serviceEndpoint Id's being a URI vs a IRI (SSI Crate limitation)
  * This changes serviceEndpoint.id from `#service` to `did:peer:#service` so that it passes Uri checks
* fix: If more than a single service was specified, then this would crash due to `#service-n` not being a valid URI
  * Changed so that all serviceEndpoint Id's are `did:peer:#service` as the starting string
* update: `tokio-tungstenite` crate updated from 0.23 to 0.24

### 9th September 2024 (release 0.1.5)

* Renaming crate names
* Setting publich to true for crates.io
* Bumping crate versions

### 5th September 2024 (release 0.1.4)

* Updated crates
* did-peer added missing types and support for peer implementation type 0 (supports 0 and 2).

### 3rd September 2024 (release 0.1.3)

* Added Debug trait to ClientConfig so we can print the config elsewhere

### 2nd September 2024 (release: 0.1.2)

* tokio crate updated
* release version changed to 0.1.2
* benchmark example - warnings removed

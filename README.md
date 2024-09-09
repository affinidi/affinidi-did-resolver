# Affinidi Trust Network - Affinidi DID Resolver

Library of useful Decentralized Identifier (DID) libraries.

## Crate Structure

- affinidi-did-resolver-cache-sdk

  - Developer friendly crate to instantiate either a local or network DID resolver with caching.
  - List of supported DID Methods is listed in the SDK README.

- affinidi-did-resolver-cache-server

  Remote server that resolves and caches DID Documents at scale.

- affinidi-did-resolver-methods

  Individual custom DID Method implementations reside here.

## Getting Started 

### I want to start resolving DID's

1. Read the affinidi-did-resolver-cache-sdk documentation, and get started with the example code.

### I want to run a production network server for scale and offloading DID method resolving?

1. Read the affinidi-did-resolver-cache-server documentation, fire it up as a service where ever you like.
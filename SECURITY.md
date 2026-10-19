# Security policy

## Reporting a vulnerability
Please **do not open a public issue**. Use GitHub's private [security advisory form](https://github.com/Nolek0/nova/security/advisories/new). You will get an answer within a few days.

## Scope
Especially interesting: anything that lets a document, web page or model output
- run commands or open files without the user's approval,
- read files outside the folders the user allowed,
- send data off the machine without the user asking,
- bypass update signature verification.

## Update integrity
Updates are signed with a private key that never leaves the maintainer's machine (and the CI secret store). The app embeds the public key and refuses unsigned or tampered packages.

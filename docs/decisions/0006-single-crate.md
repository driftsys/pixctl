# AD-0006 Single crate for now

## Context

The contract and the estimator may be reused by diagctl or tblctl.

## Options

Alternatives not documented.

## Decision

pixctl is one crate. Modules keep pure interfaces (notably `font::estimate` and
`rule::plan`) so the contract and the estimator can be extracted into crates
later, when another tool consumes them.

Satisfies: [pixctl#1](https://github.com/driftsys/pixctl/issues/1)

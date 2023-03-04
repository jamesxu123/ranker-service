# Ranker
[![Rust](https://github.com/jamesxu123/ranker-service/actions/workflows/rust.yml/badge.svg)](https://github.com/jamesxu123/ranker-service/actions/workflows/rust.yml)

Uses Glicko2 algorithm to rank and judge anything through pairwise comparisons. This project is written in Rust for fun.

## Architecture
For easy development and deployment, this app will stateful and thus should not be scaled horizontally. Some data will be persisted to database (likely SQLite), but the idea of this is to have a self-contained binary that can do it all.

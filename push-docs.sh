#!/bin/bash

set -Eeuo pipefail

rm -rf target/doc
cargo doc --no-deps
cp -r target/doc ./doc
echo '<meta http-equiv="refresh" content="0; url=interpreter_experiments">' > doc/index.html
git add doc/
git commit -m "Docs"
git push origin `git subtree split --prefix doc master`:gh-pages --force
git reset --hard HEAD~1
rm -rf doc

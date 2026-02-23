#!/bin/bash
for run in {1..100}; do
  ./target/release/examples/bench >/dev/null
done


#!/bin/bash

cat watch.txt | xargs kill -9

rm watch.txt

cargo run &

echo $! > watch.txt

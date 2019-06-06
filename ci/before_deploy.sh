#!/bin/bash

src=$(pwd)
stage=$(mktemp -d)

cp target/release/efficio-server $stage/
cp target/debug/efficio-server $stage/efficio-server-dbg
cd $stage
tar czf $src/$CRATE_NAME.tar.gz *
cd $src
rm -rf $stage

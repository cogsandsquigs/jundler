# RUN: sh -ea @file @tmpdir

# configuration

NAME=simple-bundle

# script. DON'T TOUCH (to configure!)!

TMPPATH=$1/$NAME-$RANDOM

mkdir -p $TMPPATH
cp -R tests/fixtures/$NAME/. $TMPPATH

cargo run -- $TMPPATH -b

# Ok to touch!

# CHECK: Hello, world!
$TMPPATH/$NAME

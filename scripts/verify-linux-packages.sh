#!/bin/sh
# Fresh container installs exercise real package-manager dependency resolution.
set -eu
assets="$(cd "$1" && pwd)"
docker run --rm -v "$assets:/assets:ro" debian:12-slim sh -ec '
  apt-get update -qq
  apt-get install -y /assets/*.deb
  numpad --export-demo /tmp/numpad-check
  test -s /tmp/numpad-check/sample.pdf
  test -s /tmp/numpad-check/sample.xlsx
'
docker run --rm -v "$assets:/assets:ro" fedora:43 sh -ec '
  dnf install -y /assets/*.rpm
  numpad --export-demo /tmp/numpad-check
  test -s /tmp/numpad-check/sample.pdf
'
docker run --rm -v "$assets:/assets:ro" archlinux:base sh -ec '
  pacman -Syu --noconfirm
  pacman -U --noconfirm /assets/*.pkg.tar.zst
  numpad --export-demo /tmp/numpad-check
  test -s /tmp/numpad-check/sample.pdf
'

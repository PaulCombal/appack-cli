#!/usr/bin/env bash

rm *.snap

set -e

snapcraft pack
sudo snap remove --purge appack
sudo snap install *.snap --dangerous
#sudo snap connect appack:home-all
sudo snap connect appack:kvm
sudo snap connect appack:alsa
#sudo snap connect appack:network-control
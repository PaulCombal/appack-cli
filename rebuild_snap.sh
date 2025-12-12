#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, version 3.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.


rm *.snap

set -e

snapcraft pack --verbose
sudo snap remove --purge appack
sudo snap install *.snap --dangerous
sudo snap connect appack:kvm
sudo snap connect appack:alsa
sudo snap connect appack:dot-local-share-applications

# Cheatsheet
# https://documentation.ubuntu.com/snapcraft/stable/how-to/publishing/manage-revisions-and-releases/

# Publish+upload: snapcraft upload <snap-revision>.snap --release <channel>
# Publish+upload: snapcraft upload appack_*.snap --release stable

# Upload: snapcraft upload <snap-revision>.snap
# Publish: snapcraft release <snap-name> <revision> <channel>

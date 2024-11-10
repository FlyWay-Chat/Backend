# Copyright (C) 2024  TinyBlueSapling
# This file is part of BeTalky.
# 
# BeTalky is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# 
# BeTalky is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# 
# You should have received a copy of the GNU Affero General Public License
# along with BeTalky.  If not, see <https://www.gnu.org/licenses/>.

#!/bin/bash
LOCAL_WATCHEXEC_DIR="./.watchexec"
WATCHEXEC="$LOCAL_WATCHEXEC_DIR/bin/watchexec"

# Use a global watchexec install (if available)
GLOBAL_WATCHEXEC=$(which watchexec)
if [[ -n "$GLOBAL_WATCHEXEC" ]]; then
  WATCHEXEC="$GLOBAL_WATCHEXEC"
fi

# Install watchexec locally (if needed)
if [ ! -f "$WATCHEXEC" ]; then
    echo "watchexec not found. Installing locally..."
    
    # Create the directory if it doesn't exist
    mkdir -p "$LOCAL_WATCHEXEC_DIR"
    
    # Install watchexec
    cargo install --locked watchexec-cli --root "$LOCAL_WATCHEXEC_DIR"
fi

# Run watchexec
"$WATCHEXEC" --project-origin src -e rs -r cargo run

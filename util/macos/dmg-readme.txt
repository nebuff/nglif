nglif for macOS
===============

Drag nglif into Applications, then open it from there. It runs natively on
Apple silicon and Intel Macs (macOS 10.15 or later).

The first time, macOS refuses to open it, because nglif is not notarized by
Apple (which costs a yearly fee). It has not been tampered with; to open it:

  macOS 15 or later:  open nglif once and close the warning, then go to
                      System Settings > Privacy & Security, scroll down, and
                      click "Open Anyway" next to the message about nglif.

  macOS 14 or older:  Control-click nglif in Applications, choose Open, and
                      click Open in the dialog.

  Or, in Terminal:    xattr -dr com.apple.quarantine /Applications/nglif.app

After that it opens like any other app, and Finder opens .glif and .glifjson
files in it. On a Mac, Command does what the docs call Ctrl.

Source, help and the Linux packages: https://github.com/nebuff/nglif

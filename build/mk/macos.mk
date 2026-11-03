# macOS: nglif.app with one universal binary (Apple silicon + Intel), and a .dmg to share it.
# See util/macos/build-app.sh for what gets built and checked.
.PHONY: macos-app macos-dmg
macos-app:
	util/macos/build-app.sh
macos-dmg:
	util/macos/build-app.sh --dmg

// Creates a virtual Retina (2x) display and makes it the main display, for as long as this
// process runs. CI machines have a single 1x screen, which would never exercise nglif's HiDPI
// code; with this, a window opens at 2 pixels per point like on a MacBook.
//
//   clang -fobjc-arc -framework Foundation -framework CoreGraphics virtual-display.m -o vd
//   ./vd 1440 900 &      # points; the backing store is twice that
//
// CGVirtualDisplay is private CoreGraphics API (macOS 11+), the same that screen-sharing and
// "dummy display" utilities use. It's only for tests: nglif itself never touches it.

#import <CoreGraphics/CoreGraphics.h>
#import <Foundation/Foundation.h>

@interface CGVirtualDisplayDescriptor : NSObject
@property(retain, nonatomic) NSString *name;
@property(nonatomic) unsigned int maxPixelsWide;
@property(nonatomic) unsigned int maxPixelsHigh;
@property(nonatomic) CGSize sizeInMillimeters;
@property(nonatomic) unsigned int productID;
@property(nonatomic) unsigned int vendorID;
@property(nonatomic) unsigned int serialNum;
- (void)setDispatchQueue:(dispatch_queue_t)queue;
@end

@interface CGVirtualDisplayMode : NSObject
- (instancetype)initWithWidth:(unsigned int)width height:(unsigned int)height refreshRate:(double)hz;
@end

@interface CGVirtualDisplaySettings : NSObject
@property(retain, nonatomic) NSArray *modes;
@property(nonatomic) unsigned int hiDPI;
@end

@interface CGVirtualDisplay : NSObject
@property(readonly, nonatomic) CGDirectDisplayID displayID;
- (instancetype)initWithDescriptor:(CGVirtualDisplayDescriptor *)descriptor;
- (BOOL)applySettings:(CGVirtualDisplaySettings *)settings;
@end

int main(int argc, char **argv) {
    @autoreleasepool {
        unsigned int w = argc > 1 ? (unsigned int)atoi(argv[1]) : 1440;
        unsigned int h = argc > 2 ? (unsigned int)atoi(argv[2]) : 900;
        if (!NSClassFromString(@"CGVirtualDisplay")) {
            fprintf(stderr, "virtual-display: CGVirtualDisplay is not available on this macOS\n");
            return 2;
        }

        CGVirtualDisplayDescriptor *descriptor = [CGVirtualDisplayDescriptor new];
        [descriptor setDispatchQueue:dispatch_get_main_queue()];
        descriptor.name = @"nglif test Retina display";
        descriptor.maxPixelsWide = w * 2;
        descriptor.maxPixelsHigh = h * 2;
        descriptor.sizeInMillimeters = CGSizeMake(w * 0.2, h * 0.2);  // ~254 ppi, laptop-like
        descriptor.vendorID = 0x6e67;  // "ng"
        descriptor.productID = 0x6c66; // "lf"
        descriptor.serialNum = 1;

        CGVirtualDisplay *display = [[CGVirtualDisplay alloc] initWithDescriptor:descriptor];
        CGVirtualDisplaySettings *settings = [CGVirtualDisplaySettings new];
        settings.hiDPI = 1;
        settings.modes = @[ [[CGVirtualDisplayMode alloc] initWithWidth:w height:h refreshRate:60] ];
        if (!display || ![display applySettings:settings]) {
            fprintf(stderr, "virtual-display: could not create a %ux%u HiDPI display\n", w, h);
            return 1;
        }
        CGDirectDisplayID vid = display.displayID;

        // Wait for the window server to bring it online, then put it at the origin so it
        // becomes the main display, where new windows (and the menu bar) appear.
        for (int i = 0; i < 100 && !CGDisplayIsOnline(vid); i++) usleep(50000);
        CGDirectDisplayID old = CGMainDisplayID();
        // Some virtual machines ignore a configuration that only lasts as long as this process,
        // so fall back to one for the login session (CI machines are thrown away afterwards).
        CGConfigureOption options[] = {kCGConfigureForAppOnly, kCGConfigureForSession};
        for (int attempt = 0; attempt < 2 && CGMainDisplayID() != vid; attempt++) {
            CGDisplayConfigRef config;
            if (CGBeginDisplayConfiguration(&config) != kCGErrorSuccess) break;
            CGConfigureDisplayOrigin(config, vid, 0, 0);
            if (old != vid) CGConfigureDisplayOrigin(config, old, (int32_t)w, 0);
            CGCompleteDisplayConfiguration(config, options[attempt]);
            for (int i = 0; i < 30 && CGMainDisplayID() != vid; i++) usleep(100000);
        }
        size_t pw = CGDisplayPixelsWide(vid), ph = CGDisplayPixelsHigh(vid);
        CGDisplayModeRef mode = CGDisplayCopyDisplayMode(vid);
        printf("virtual-display: id %u, %zux%zu points, %zux%zu pixels, main=%s\n", vid, pw, ph,
               mode ? CGDisplayModeGetPixelWidth(mode) : 0, mode ? CGDisplayModeGetPixelHeight(mode) : 0,
               CGMainDisplayID() == vid ? "yes" : "no");
        if (mode) CGDisplayModeRelease(mode);
        fflush(stdout);

        // Keep the display alive until killed.
        dispatch_main();
    }
}

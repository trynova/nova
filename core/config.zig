const std = @import("std");

const dist_path = "_dist";

pub const AppConfig = struct {
    dirs: AppConfigDirs,

    pub const AppConfigDirs = struct {
        cwd: std.fs.Dir,
        dist: std.fs.Dir,
        pages: std.fs.Dir,

        pub fn init() !AppConfigDirs {
            var cwd = std.fs.cwd();
            errdefer cwd.close();

            var dist = cwd.openDir(dist_path, .{}) catch |e|
                if (e == error.FileNotFound) try cwd.makeOpenPath(dist_path, .{}) else return e;
            errdefer dist.close();

            const pages = try cwd.openDir("pages", .{ .iterate = true });

            return AppConfigDirs{ .cwd = cwd, .dist = dist, .pages = pages };
        }

        pub fn closeAll(dirs: *AppConfigDirs) void {
            dirs.cwd.close();
            dirs.dist.close();
            dirs.pages.close();
        }
    };

    pub fn init() !AppConfig {
        return AppConfig{ .dirs = try AppConfigDirs.init() };
    }
};

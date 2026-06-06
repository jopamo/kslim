#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn git_in(dir: &str, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|_| panic!("failed to run git {:?} in {}", args, dir));
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        panic!("git {:?} failed in {}: {}", args, dir, stderr);
    }
    stdout
}

pub fn create_fake_upstream(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    std::fs::create_dir_all(&work).unwrap();
    for dir in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    for d in &[
        "arch", "drivers", "fs", "include", "kernel", "mm", "net", "scripts",
    ] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_amdgpu(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers/gpu/drm/amd/amdgpu",
        "fs",
        "include",
        "kernel",
        "mm",
        "net",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Kconfig"),
        "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Makefile"),
        "obj-$(CONFIG_DRM_AMDGPU) += amd/amdgpu/\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        r#"config DRM_AMDGPU
	tristate "AMD GPU"
	depends on DRM
	help
	  Choose this option if you have an AMD GPU.

config DRM_AMDGPU_SI
	bool "Southern Islands"
	depends on DRM_AMDGPU
"#,
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Makefile"),
        "amdgpu-y := amdgpu_drv.o \\\n helper.o\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
        "int amdgpu_drv;\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/helper.c"),
        "int amdgpu_helper;\n",
    )
    .unwrap();
    for d in &["arch", "fs", "include", "kernel", "mm", "net", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_gpu_siblings(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers/gpu/drm/amd/amdgpu",
        "drivers/gpu/drm/nouveau",
        "fs",
        "include",
        "kernel",
        "mm",
        "net",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "source \"drivers/gpu/drm/Kconfig\"\n").unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Kconfig"),
        concat!(
            "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
            "source \"drivers/gpu/drm/nouveau/Kconfig\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Makefile"),
        "obj-$(CONFIG_DRM_AMDGPU) += amd/amdgpu/\nobj-$(CONFIG_DRM_NOUVEAU) += nouveau/\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        "config DRM_AMDGPU\n\ttristate \"AMD GPU\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
        "int amdgpu_drv;\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/nouveau/Kconfig"),
        "config DRM_NOUVEAU\n\ttristate \"Nouveau\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/nouveau/nouveau_drv.c"),
        "int nouveau_drv;\n",
    )
    .unwrap();
    for d in &["arch", "fs", "include", "kernel", "mm", "net", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_rxrpc_afs(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers",
        "fs/afs",
        "include",
        "kernel",
        "mm",
        "net/rxrpc",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(work.join("net/Kconfig"), "source \"net/rxrpc/Kconfig\"\n").unwrap();
    std::fs::write(
        work.join("net/Makefile"),
        "obj-$(CONFIG_AF_RXRPC) += rxrpc/\n",
    )
    .unwrap();
    std::fs::write(
        work.join("net/rxrpc/Kconfig"),
        "config AF_RXRPC\n\ttristate \"RxRPC sockets\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("net/rxrpc/Makefile"),
        "af-rxrpc-y := call.o\nobj-y += af-rxrpc.o\n",
    )
    .unwrap();
    std::fs::write(work.join("net/rxrpc/call.c"), "int rxrpc_call;\n").unwrap();
    std::fs::write(work.join("fs/Kconfig"), "source \"fs/afs/Kconfig\"\n").unwrap();
    std::fs::write(work.join("fs/Makefile"), "obj-$(CONFIG_AFS_FS) += afs/\n").unwrap();
    std::fs::write(
        work.join("fs/afs/Kconfig"),
        concat!(
            "config AFS_FS\n",
            "\ttristate \"AFS support\"\n",
            "\tselect AF_RXRPC\n",
        ),
    )
    .unwrap();
    std::fs::write(
        work.join("fs/afs/Makefile"),
        "afs-y := super.o\nobj-y += afs.o\n",
    )
    .unwrap();
    std::fs::write(work.join("fs/afs/super.c"), "int afs_super;\n").unwrap();
    for d in &["arch", "drivers", "include", "kernel", "mm", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_nfs_stack(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers",
        "fs/lockd",
        "fs/nfs",
        "fs/nfs_common",
        "fs/nfsd",
        "include/linux",
        "include/uapi/linux",
        "kernel",
        "mm",
        "net/sunrpc",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(
        work.join("fs/Kconfig"),
        concat!(
            "source \"fs/nfs/Kconfig\"\n",
            "source \"fs/nfsd/Kconfig\"\n",
            "source \"fs/lockd/Kconfig\"\n",
            "source \"fs/nfs_common/Kconfig\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        work.join("fs/Makefile"),
        concat!(
            "obj-$(CONFIG_NFS_FS) += nfs/\n",
            "obj-$(CONFIG_NFSD) += nfsd/\n",
            "obj-$(CONFIG_LOCKD) += lockd/\n",
            "obj-$(CONFIG_NFS_COMMON) += nfs_common/\n",
        ),
    )
    .unwrap();
    std::fs::write(work.join("net/Kconfig"), "source \"net/sunrpc/Kconfig\"\n").unwrap();
    std::fs::write(
        work.join("net/Makefile"),
        "obj-$(CONFIG_SUNRPC) += sunrpc/\n",
    )
    .unwrap();
    std::fs::write(
        work.join("net/sunrpc/Kconfig"),
        "config SUNRPC\n\ttristate \"SUNRPC\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("fs/lockd/Kconfig"),
        "config LOCKD\n\ttristate \"Lockd\"\n\tdepends on SUNRPC\n",
    )
    .unwrap();
    std::fs::write(
        work.join("fs/nfs_common/Kconfig"),
        "config NFS_COMMON\n\ttristate \"NFS common\"\n\tdepends on SUNRPC\n",
    )
    .unwrap();
    std::fs::write(
        work.join("fs/nfs/Kconfig"),
        "config NFS_FS\n\ttristate \"NFS client\"\n\tdepends on SUNRPC && NFS_COMMON\n",
    )
    .unwrap();
    std::fs::write(
        work.join("fs/nfsd/Kconfig"),
        "config NFSD\n\ttristate \"NFS server\"\n\tdepends on SUNRPC && LOCKD\n",
    )
    .unwrap();

    std::fs::write(work.join("net/sunrpc/Makefile"), "sunrpc-y := clnt.o\n").unwrap();
    std::fs::write(work.join("net/sunrpc/clnt.c"), "int sunrpc_client;\n").unwrap();
    std::fs::write(
        work.join("net/sunrpc/sunrpc.h"),
        "#define SUNRPC_PRIVATE 1\n",
    )
    .unwrap();
    std::fs::write(work.join("fs/lockd/Makefile"), "lockd-y := svc.o\n").unwrap();
    std::fs::write(work.join("fs/lockd/svc.c"), "int lockd_svc;\n").unwrap();
    std::fs::write(
        work.join("fs/nfs_common/Makefile"),
        "nfs_common-y := common.o\n",
    )
    .unwrap();
    std::fs::write(work.join("fs/nfs_common/common.c"), "int nfs_common;\n").unwrap();
    std::fs::write(work.join("fs/nfs/Makefile"), "nfs-y := client.o\n").unwrap();
    std::fs::write(work.join("fs/nfs/client.c"), "int nfs_client;\n").unwrap();
    std::fs::write(work.join("fs/nfs/internal.h"), "#define NFS_PRIVATE 1\n").unwrap();
    std::fs::write(work.join("fs/nfsd/Makefile"), "nfsd-y := server.o\n").unwrap();
    std::fs::write(work.join("fs/nfsd/server.c"), "int nfsd_server;\n").unwrap();
    std::fs::write(
        work.join("include/linux/nfs_fs.h"),
        "#define NFS_PUBLIC_HEADER 1\n",
    )
    .unwrap();
    std::fs::write(
        work.join("include/uapi/linux/nfs_mount.h"),
        "#define NFS_UAPI_HEADER 1\n",
    )
    .unwrap();

    for d in &["arch", "drivers", "kernel", "mm", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_preprocessor_fixture(
    tmp: &Path,
    name: &str,
    version: &str,
) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers/test",
        "fs",
        "include",
        "kernel",
        "mm",
        "net",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(
        work.join("drivers/test/feature.c"),
        concat!(
            "#ifdef CONFIG_TEST_REMOVED\n",
            "int removed_branch;\n",
            "#else\n",
            "int live_branch;\n",
            "#endif\n",
            "\n",
            "#if defined(CONFIG_TEST_REMOVED) || defined(CONFIG_OTHER)\n",
            "int unsupported_expression;\n",
            "#endif\n",
        ),
    )
    .unwrap();
    for d in &["arch", "fs", "include", "kernel", "mm", "net", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_include_fixture(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers/gpu/drm",
        "drivers/gpu/drm/amd/amdgpu",
        "fs",
        "include/linux",
        "kernel",
        "mm",
        "net",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Kconfig"),
        "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Makefile"),
        "obj-$(CONFIG_DRM_AMDGPU) += amd/amdgpu/\nobj-y += helper.o\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        "config DRM_AMDGPU\n\ttristate \"AMD GPU\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Makefile"),
        "amdgpu-y := amdgpu_drv.o\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
        "int amdgpu_drv;\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/internal.h"),
        "#define AMDGPU_PRIVATE 1\n",
    )
    .unwrap();
    std::fs::write(
        work.join("include/linux/drm_public.h"),
        "#define DRM_PUBLIC 1\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/helper.c"),
        concat!(
            "#include <amd/amdgpu/internal.h>\n",
            "#include <linux/drm_public.h>\n",
            "int drm_helper;\n",
        ),
    )
    .unwrap();
    let fake_make = work.join("fake-make.sh");
    std::fs::write(
        &fake_make,
        concat!(
            "#!/bin/sh\n",
            "if grep -q '^#include <amd/amdgpu/internal.h>$' drivers/gpu/drm/helper.c; then\n",
            "  printf '%s\\n' 'drivers/gpu/drm/helper.c:1:10: fatal error: amd/amdgpu/internal.h: No such file or directory' >&2\n",
            "  exit 2\n",
            "fi\n",
            "exit 0\n",
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&fake_make).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_make, perms).unwrap();
    }
    for d in &["arch", "fs", "kernel", "mm", "net", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_amdgpu_hard_stop(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers/gpu/drm/amd/amdgpu",
        "fs",
        "include",
        "kernel",
        "mm",
        "net",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Kconfig"),
        "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Makefile"),
        "obj-$(CONFIG_DRM_AMDGPU) += amd/amdgpu/\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        "config DRM_AMDGPU\n\ttristate \"AMD GPU\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Makefile"),
        "amdgpu-y := amdgpu_drv.o\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
        "int amdgpu_drv;\n",
    )
    .unwrap();
    let fake_make = work.join("fake-make.sh");
    std::fs::write(
        &fake_make,
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' \"$*\" >> .kslim-selftest.log\n",
            "printf '%s\\n' 'drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c:1:10: fatal error: generated/missing.h: No such file or directory' >&2\n",
            "exit 2\n",
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&fake_make).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_make, perms).unwrap();
    }
    for d in &["arch", "fs", "include", "kernel", "mm", "net", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_missing_header_diag_sequence(
    tmp: &Path,
    name: &str,
    version: &str,
    headers: &[&str],
) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers/gpu/drm/amd/amdgpu",
        "fs",
        "include",
        "kernel",
        "mm",
        "net",
        "scripts",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Kconfig"),
        "source \"drivers/gpu/drm/amd/amdgpu/Kconfig\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/Makefile"),
        "obj-$(CONFIG_DRM_AMDGPU) += amd/amdgpu/\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Kconfig"),
        "config DRM_AMDGPU\n\ttristate \"AMD GPU\"\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/Makefile"),
        "amdgpu-y := amdgpu_drv.o\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/gpu/drm/amd/amdgpu/amdgpu_drv.c"),
        "int amdgpu_drv;\n",
    )
    .unwrap();
    let mut helper = String::new();
    for header in headers {
        helper.push_str(&format!("#include <{}>\n", header));
    }
    helper.push_str("int drm_helper;\n");
    std::fs::write(work.join("drivers/gpu/drm/helper.c"), helper).unwrap();
    let fake_make = work.join("fake-make.sh");
    let mut fake_make_script = String::from(concat!(
        "#!/bin/sh\n",
        "printf '%s\\n' \"$*\" >> .kslim-selftest.log\n",
    ));
    for header in headers {
        fake_make_script.push_str(&format!(
            concat!(
                "if grep -q '^#include <{}>$' drivers/gpu/drm/helper.c; then\n",
                "  printf '%s\\n' 'drivers/gpu/drm/helper.c:1:10: fatal error: {}: No such file or directory' >&2\n",
                "  exit 2\n",
                "fi\n",
            ),
            header, header
        ));
    }
    fake_make_script.push_str("exit 0\n");
    std::fs::write(&fake_make, fake_make_script).unwrap();
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&fake_make).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_make, perms).unwrap();
    }
    for d in &["arch", "fs", "include", "kernel", "mm", "net", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_upstream_with_known_missing_header_diag(
    tmp: &Path,
    name: &str,
    version: &str,
) -> String {
    create_fake_upstream_with_missing_header_diag_sequence(
        tmp,
        name,
        version,
        &["amd/amdgpu/amdgpu_missing.h"],
    )
}

pub fn create_fake_upstream_with_realtek(tmp: &Path, name: &str, version: &str) -> String {
    let work = tmp.join(format!("upstream-work-{}", name));
    let bare = tmp.join(format!("upstream-{}.git", name));

    for dir in &[
        "arch",
        "drivers/net/ethernet/realtek/rtase",
        "fs",
        "include",
        "kernel",
        "mm",
        "net",
        "scripts",
        "tools/testing/selftests/drivers/net",
    ] {
        std::fs::create_dir_all(work.join(dir)).unwrap();
    }
    std::fs::write(
        work.join("Makefile"),
        format!("# Linux {} Makefile\n", version),
    )
    .unwrap();
    std::fs::write(work.join("Kconfig"), "# Kconfig\n").unwrap();
    std::fs::write(
        work.join("drivers/net/ethernet/realtek/Kconfig"),
        "config RTASE\n\ttristate \"RTASE\"\nendif # NET_VENDOR_REALTEK\n",
    )
    .unwrap();
    std::fs::write(
        work.join("drivers/net/ethernet/realtek/Makefile"),
        "obj-$(CONFIG_RTASE) += rtase/\n",
    )
    .unwrap();
    std::fs::write(work.join("drivers/net/ethernet/realtek/rtase/.keep"), "").unwrap();
    std::fs::write(
        work.join("tools/testing/selftests/drivers/net/Makefile"),
        "# selftests\n",
    )
    .unwrap();
    for d in &["arch", "fs", "include", "kernel", "mm", "net", "scripts"] {
        std::fs::write(work.join(d).join(".keep"), "").unwrap();
    }

    git_in(work.to_str().unwrap(), &["init"]);
    git_in(
        work.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        work.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(work.to_str().unwrap(), &["add", "-A"]);
    git_in(
        work.to_str().unwrap(),
        &["commit", "-m", &format!("Linux {}", version)],
    );
    git_in(work.to_str().unwrap(), &["tag", &format!("v{}", version)]);
    git_in(
        tmp.to_str().unwrap(),
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git_in(
        bare.to_str().unwrap(),
        &["remote", "set-url", "origin", bare.to_str().unwrap()],
    );

    bare.to_str().unwrap().to_string()
}

pub fn create_fake_rtlmq_source(tmp: &Path) -> std::path::PathBuf {
    let root = tmp.join("eth");
    let src = root.join("rtlmq");
    let tests = root.join("rtlmq-tests");
    let selftests = tests.join("selftests/drivers/net/rtlmq");

    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(src.join("scripts")).unwrap();
    std::fs::create_dir_all(&selftests).unwrap();

    std::fs::write(
        src.join("Makefile"),
        "obj-$(CONFIG_RTLMQ) += rtlmq.o\nrtlmq-y += rtlmq_main.o\n",
    )
    .unwrap();
    std::fs::write(src.join("Kconfig"), "config RTLMQ\n\ttristate \"RTLMQ\"\n").unwrap();
    std::fs::write(src.join("rtlmq_main.c"), "int rtlmq_main;\n").unwrap();
    std::fs::write(src.join("rtlmq_hw.h"), "#define RTLMQ_HW 1\n").unwrap();
    std::fs::write(src.join("scripts/helper.sh"), "#!/bin/sh\nexit 0\n").unwrap();

    std::fs::create_dir_all(&tests).unwrap();
    std::fs::write(tests.join("rtlmq_kunit_refine.c"), "int rtlmq_test;\n").unwrap();
    std::fs::write(tests.join("rtlmq_kunit_shared.h"), "#define RTLMQ_TEST 1\n").unwrap();
    std::fs::write(tests.join(".kunitconfig"), "CONFIG_RTLMQ_KUNIT_TEST=y\n").unwrap();
    std::fs::write(tests.join("TESTING.rst"), "rtlmq testing\n").unwrap();
    std::fs::write(selftests.join("smoke.sh"), "#!/bin/sh\nexit 0\n").unwrap();

    src
}

pub fn create_kslim_project(
    tmp: &Path,
    project_name: &str,
    output_path: &str,
    upstream_path: &str,
) -> std::path::PathBuf {
    let kslim_dir = tmp.join("kslim");
    std::fs::create_dir_all(&kslim_dir).unwrap();

    let config = format!(
        r#"[project]
name = "{name}"

[upstream]
name = "linux"
url = "{up}"

[output]
path = "{out}"
branch_prefix = "kslim"
"#,
        name = project_name,
        up = upstream_path,
        out = output_path,
    );
    std::fs::write(kslim_dir.join("kslim.toml"), config).unwrap();

    std::fs::create_dir_all(kslim_dir.join("profiles")).unwrap();
    let profile = r#"[profile]
name = "default"
description = "Test profile"

[base]
ref = "v1.0"
"#;
    std::fs::write(kslim_dir.join("profiles/default.toml"), profile).unwrap();
    std::fs::create_dir_all(kslim_dir.join("manifests")).unwrap();
    kslim_dir
}

pub fn create_patch_worktree(
    tmp: &Path,
    upstream_path: &str,
    branch: &str,
    file: &str,
    replacement: &str,
) -> std::path::PathBuf {
    create_patch_worktree_replace(tmp, upstream_path, branch, file, "1.0", replacement)
}

pub fn create_patch_worktree_replace(
    tmp: &Path,
    upstream_path: &str,
    branch: &str,
    file: &str,
    needle: &str,
    replacement: &str,
) -> std::path::PathBuf {
    let repo = tmp.join(format!("patch-worktree-{}", branch.replace('/', "-")));
    git_in(
        tmp.to_str().unwrap(),
        &["clone", upstream_path, repo.to_str().unwrap()],
    );
    git_in(
        repo.to_str().unwrap(),
        &["config", "user.email", "test@kslim.local"],
    );
    git_in(
        repo.to_str().unwrap(),
        &["config", "user.name", "kslim test"],
    );
    git_in(repo.to_str().unwrap(), &["checkout", "-b", branch]);

    let path = repo.join(file);
    let content = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, content.replace(needle, replacement)).unwrap();
    git_in(repo.to_str().unwrap(), &["add", "-A"]);
    git_in(
        repo.to_str().unwrap(),
        &["commit", "-m", &format!("Patch {}", branch)],
    );
    repo
}

pub fn output_meta_path(output_dir: &Path, rel: &str) -> std::path::PathBuf {
    output_dir.join(".git/kslim").join(rel)
}

pub fn committed_output_meta_path(output_dir: &Path, rel: &str) -> std::path::PathBuf {
    output_dir.join(".kslim").join(rel)
}

pub fn project_failure_report_path(kslim_dir: &Path) -> std::path::PathBuf {
    project_failure_meta_path(kslim_dir, "report.txt")
}

pub fn project_failure_meta_path(kslim_dir: &Path, rel: &str) -> std::path::PathBuf {
    kslim_dir.join(".kslim/attempt").join(rel)
}

pub fn kslim_bin() -> String {
    let test_exe = std::env::current_exe().unwrap();
    let deps_dir = test_exe.parent().unwrap();
    let target_dir = deps_dir.parent().unwrap();
    let bin = target_dir.join("kslim");
    if bin.exists() {
        return bin.to_str().unwrap().to_string();
    }
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_kslim") {
        return path;
    }
    panic!("could not find kslim binary");
}

pub fn kslim_in(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let bin = kslim_bin();
    let output = Command::new(&bin)
        .args(args)
        .current_dir(dir)
        .env("RUST_LOG", "off")
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "failed to run kslim at '{}' in '{}': {}",
                bin,
                dir.display(),
                e
            )
        });
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

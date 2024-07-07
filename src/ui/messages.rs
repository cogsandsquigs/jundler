pub const MAX_MSG_LEN: usize = 49;

pub const WELCOME_MSG: &str = "✨ Welcome to jundler! ✨";
pub const INIT_BUILD_MSG: &str = "⏳ Building...";
pub const INIT_CLEAN_MSG: &str = "⏳ Cleaning...";
pub const CLEAN_CACHE_MSG: &str = "🧹 Cleaning cache";
pub const COPY_PROJ_MSG: &str = "📥 Copying project and preparing for build";
pub const BUNDLE_PROJ_MSG: &str = "📦 Bundling project with ESBuild";
pub const ESBUILD_BINARY_MSG: &str = "🔎 Retrieving ESBuild binary";
pub const BUNDLING_MSG: &str = "📦 Bundling";
pub const HOST_NODE_MSG: &str = "🔎 Retrieving Host Node.js binary";
pub const TARGET_NODE_MSG: &str = "🔎 Retrieving Target Node.js binary";
pub const GEN_SEA_BLOB_MSG: &str = "🧪 Generating SEA blob";
pub const INJECT_APP_MSG: &str = "💉 Injecting application into Node.js binary";
pub const MACOS_CODESIGN_MSG: &str = "🔏 Codesigning macOS binary";
pub const WINDOWS_CODESIGN_MSG: &str = "🔏 Codesigning Windows binary";

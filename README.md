[![License](https://img.shields.io/github/license/cogsandsquigs/jundler?style=for-the-badge)](https://github.com/cogsandsquigs/jundler/blob/main/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/jundler?style=for-the-badge)](https://crates.io/crates/jundler)
[![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/jundler?style=for-the-badge)](https://crates.io/crates/jundler)

# Jundler

> The **J**avaScript executable b**undler** for Node.js projects.

Jundler is a tool that bundles your Node.js project into a single executable file. It uses the new [Single Executable Application API](https://nodejs.org/api/single-executable-applications.html) to bundle your project, which can then be run on any platform without needing to install Node.js or any dependencies.

## Why Jundler?

Jundler, unlike other executable bundlers (like [pkg](https://github.com/vercel/pkg), [boxednode](https://github.com/mongodb-js/boxednode), or [js2bin](https://github.com/criblio/js2bin)), uses the latest [Single Executable Application API](https://nodejs.org/api/single-executable-applications.html). And, unlike other bundlers, Jundler is written in Rust, which makes it faster and more efficient!

For more features, see the [FAQ](#faq).

## Requirements

-   An installation of `node`, `npm` and `npx`. Note that the version of `node` you're using must match the version you intend on bundling with Jundler. You can check your version of `node` by running `node -v` in your terminal.
-   **OPTIONAL:** `cargo` to install from `crates.io` directly instead of using the binaries.

## Usage

Jundler will automatically bundle your JavaScript files into a single standalone file. You will need to both a [`sea-config.json`](https://nodejs.org/api/single-executable-applications.html#generating-single-executable-preparation-blobs) file and the ubiquitous `package.json` file in the root of your project.

To use Jundler, simply run the following command in your terminal:

```bash
jundler <path-to-nodejs-project>
```

Run `jundler --help` for more information on how to use Jundler.

## FAQ

### Wait! Something broke! What do I do?

Because both Jundler and the [Single Executable Application API](https://nodejs.org/api/single-executable-applications.html) are new and changing rapidly, things can break overnight. If something breaks, please open an issue on the [Jundler GitHub repository](https://github.com/cogsandsquigs/jundler/issues) and I'll get back to you when feasable.

### I'm getting an import error when I run my bundled executable. What do I do?

This is a known issue with the Single Executable Application API, as it does not support `import` or `require`. To fix this, just tell Jundler to bundle your project with the `--bundle`/`-b` flag.

```bash
jundler <path-to-nodejs-project> --bundle
```

### Does Jundler support TypeScript?

Yup. Just specify the `--bundle`/`-b` flag when running Jundler --- ESBuild does the rest!

### Does Jundler support cross-compilation?

Yes! Just specify the OS and architecture you want to build using the `-o` and `-a` flags respectively.

### Does Jundler support codesigning for macOS?

Yes, so long as you're on a macOS machine yourself. Jundler will automatically codesign your executable without any additional input from you!

> [!NOTE]
> If you're building for macOS on a different platform, the binary will have to be manually signed on a macOS machine. Jundler should give you a warning about this.

### Does Jundler support codesigning for Windows?

Not yet, but since windows doesn't require codesigning for binaries to run (it will just give you a warning), this should be fine for now. All Windows binaries need to be manually signed on a Windows machine after being built.

### Why the name "Jundler"?

Because it sounded funny and I liked it. :p

## TODO

-   [x] Auto-detect TS / Modules and bundle accordingly
-   [x] Unit test separate build steps
-   [ ] Integration testing on test projects
-   [ ] Spinners/progress bars + better UI
-   [ ] Better error handling
-   [ ] Auto-download of Node.js host runtime to make the process work w/o Node.js installed

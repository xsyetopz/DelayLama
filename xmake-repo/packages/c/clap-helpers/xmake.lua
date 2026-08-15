package("clap-helpers")
    set_kind("library", {headeronly = true})
    set_homepage("https://github.com/free-audio/clap-helpers")
    set_description("C++ helpers for the CLAP API")
    set_license("MIT")

    -- Match the helper revision selected by the adapter.
    add_urls("https://github.com/free-audio/clap-helpers/archive/a61bcdf0ecc2c8db1e80bfe8bf9cb7e8d9fd2bbc.tar.gz")
    add_versions("2026.8.5", "48d24c9ad5e22b36040c500198a979aa25d2d7005d76f98a9edbbee0b86a102d")
    add_includedirs("include")

    on_install(function (package)
        os.cp("include", package:installdir())
        os.cp("LICENSE", package:installdir("share/licenses/clap-helpers"))
    end)

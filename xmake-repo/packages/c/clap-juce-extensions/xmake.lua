package("clap-juce-extensions")
    set_kind("library", {headeronly = true})
    set_homepage("https://github.com/free-audio/clap-juce-extensions")
    set_description("Unofficial forkless CLAP client for JUCE AudioProcessor")
    set_license("MIT")

    -- Pin mainline because numbered tags predate CLAP 1.x and JUCE 8.
    add_urls("https://github.com/free-audio/clap-juce-extensions/archive/c1a5ad025f95d01e03267857fa8276ebeed16500.tar.gz")
    add_versions("2026.8.5", "63819da2ef9bcc520f8c3c5597ef97399ad7925863a925ac7746f8836c8739a1")
    add_includedirs("include")

    on_install(function (package)
        os.cp("include", package:installdir())
        os.cp("src", package:installdir())
        os.cp("LICENSE.md", package:installdir("share/licenses/clap-juce-extensions"))
    end)

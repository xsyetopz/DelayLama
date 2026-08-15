package("clap")
    set_kind("library", {headeronly = true})
    set_homepage("https://cleveraudio.org/")
    set_description("CLever Audio Plug-in API headers")
    set_license("MIT")

    -- Pin the adapter-compatible API commit instead of a moving branch.
    add_urls("https://github.com/free-audio/clap/archive/29ffcc273be7c7c651f6c9953b99e69700e2387a.tar.gz")
    add_versions("1.2.7", "f8c9aaf318b00e989111630b4cf1118b235bec2059904c3cd1806e71b8e37ff0")
    add_includedirs("include")

    on_install(function (package)
        os.cp("include", package:installdir())
        os.cp("LICENSE", package:installdir("share/licenses/clap"))
    end)

const storedLocale = localStorage.getItem("locale");

requirejs.config({
    baseUrl: "./",
    paths: {
        jquery: "js/lib/jquery-3.5.1.min",
        json: "js/lib/require/json",
        css: "js/lib/require/css.min",
        text: "js/lib/text",
        i18n: "js/lib/i18n",
        lib: "js/lib/",
        style: "../../css",
        app: "../../js/app",
    },
    config: {
        i18n: {
            locale: storedLocale,
        },
    },
    shim: {
        "js/lib/lobibox.min.js": {
            deps: ["jquery"],
        },
        "js/lib/d3.js": {
            exports: "d3",
        },
        "js/lib/epoch.js": {
            deps: ["js/lib/d3.js"],
            exports: "jQuery.fn.epoch",
        },
    },
});
requirejs(["app/main"]);

requirejs.config({
    baseUrl: './',
    paths: {
        jquery: 'js/lib/jquery-3.5.1.min',
        json: 'js/lib/require/json',
        css: 'js/lib/require/css.min',
        text: 'js/lib/text',
        i18n: 'js/lib/i18n',
        lib: "js/lib/",
        style: "../../css",
        app: "../../js/app"
    },
    shim: {
        'js/lib/lobibox.min.js': {
            deps: ['jquery']
        }
    }
});
requirejs(["app/main"]);
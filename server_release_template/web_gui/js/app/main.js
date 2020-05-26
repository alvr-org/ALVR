define([
    "jquery",
    "lib/bootstrap.bundle.min",
    "lib/lodash",
    "app/alvrSettings",
    "js/lib/lobibox.min.js",
    "css!js/lib/lobibox.min.css"


], function ($, bootstrap, _, ALVRSettings) {
    $(function () {
        new ALVRSettings();
    });
});
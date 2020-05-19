define([
    "jquery",
    "lib/bootstrap.bundle.min",
    "lib/lodash",
    "app/alvrSettings"


], function ($, bootstrap, _, ALVRSettings) {
    $(function () {
        new ALVRSettings();
    });
});
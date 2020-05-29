define([
    "jquery",
    "lib/bootstrap.bundle.min",
    "lib/lodash",
    "app/alvrSettings",
    "app/setupWizard",
    "json!../../session",
    "js/lib/lobibox.min.js",
    "css!js/lib/lobibox.min.css"


], function ($, bootstrap, _, ALVRSettings, SetupWizard, session) {
    $(function () {
        new ALVRSettings();

        new SetupWizard();
    });
});
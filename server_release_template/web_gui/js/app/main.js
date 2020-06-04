define([
    "jquery",
    "lib/bootstrap.bundle.min",
    "lib/lodash",
    "text!app/templates/main.html",
    "i18n!app/nls/main",
    "app/alvrSettings",
    "app/setupWizard",
    "json!../../session",
    "js/lib/lobibox.min.js",
    "css!js/lib/lobibox.min.css"


], function ($, bootstrap, _, mainTemplate, i18n, ALVRSettings, SetupWizard, session) {
    $(function () {

        var compiledTemplate = _.template(mainTemplate);
        var template = compiledTemplate({
            "title": i18n.title,
        });

        $("#bodyContent").append(template);
        $(document).ready(() => {
            var settings = new ALVRSettings();
            var wizard = new SetupWizard(settings);


            if (session.setupWizard) {
                wizard.showWizard();
            }
            $("#runSetupWizard").click(() => {
                wizard.showWizard();
            })
        });

















    });
});
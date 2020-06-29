define([
    "jquery",
    "lib/bootstrap.bundle.min",
    "lib/lodash",
    "text!app/templates/main.html",
    "i18n!app/nls/main",
    "app/settings",
    "app/setupWizard",
    "app/monitor",
    "json!../../session",
    "app/monitor",
    "js/lib/lobibox.min.js",
    "css!js/lib/lobibox.min.css"


], function ($, bootstrap, _, mainTemplate, i18n, Settings, SetupWizard, Monitor, session) {
    $(function () {

        var compiledTemplate = _.template(mainTemplate);
        var template = compiledTemplate(i18n);
       
        $("#bodyContent").append(template);       
        $(document).ready(() => {
            $('#loading').remove();          

            var settings = new Settings();
            var wizard = new SetupWizard(settings);
            var monitor = new Monitor(settings);

            $("#bodyContent").show();

            if (session.setupWizard) {
                wizard.showWizard();
            }
            $("#runSetupWizard").click(() => {
                wizard.showWizard();
            })
        });
    });
});
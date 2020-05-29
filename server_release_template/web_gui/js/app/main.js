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
        var settings = new ALVRSettings();

        var wizard = new SetupWizard(settings);

        if(session.setupWizard) {
            wizard.showWizard();
        }   
        $("#runSetupWizard").click(()=> {
            wizard.showWizard();
        })   
       
    });
});
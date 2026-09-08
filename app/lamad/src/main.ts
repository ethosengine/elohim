import { bootstrapApplication } from "@angular/platform-browser";
import "elohim-core/register"; // Side-effect: registers <elohim-page-chrome>, <elohim-epr-link>, etc.
import "elohim-imagodei/register"; // Side-effect: registers <elohim-contributor-card> (Contributors section), etc.

import { AppComponent } from "./app/app.component";
import { appConfig } from "./app/app.config";

bootstrapApplication(AppComponent, appConfig)
  .then((app) => {
    // Browser-only proof: server-rendered HTML cannot certify client bootstrap.
    for (const component of app.components) {
      const root = component.location.nativeElement as HTMLElement;
      root.setAttribute("data-app-ready", "true");
    }
  })
  .catch((err: unknown) => console.error("Application bootstrap failed:", err));

import { Component, OnInit, inject } from '@angular/core';
import { ActivatedRoute } from '@angular/router';

import { ILamadEprNav, LAMAD_EPR_NAV } from '../../interfaces/cross-pillar.interface';

/**
 * Legacy URL bridge (§12.6 Slice 2): /lamad/resource/{id} was the monolith-era
 * canonical content URL — real shares exist. The viewer is now shell-owned at
 * the universal /epr/{id} address; this route hands off across the bundle
 * boundary. (Replaces the absolute redirectTo that could never escape this
 * router and self-looped.)
 */
@Component({
  selector: 'app-legacy-resource-redirect',
  standalone: true,
  template: '',
})
export class LegacyResourceRedirectComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly eprNav: ILamadEprNav = inject(LAMAD_EPR_NAV);

  ngOnInit(): void {
    const id = this.route.snapshot.params['resourceId'] as string;
    this.eprNav.navigate(`/epr/${encodeURIComponent(id)}`); // route-literal-ok: sanctioned EprNavService universal /epr/{id} nav (legacy /resource bridge handoff), not a raw literal mint
  }
}

import { Component, OnInit, OnDestroy } from '@angular/core';
import { RouterOutlet, RouterLink, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Subject } from 'rxjs';
import { filter, takeUntil } from 'rxjs/operators';
import { DataLoaderService } from '../../services/data-loader.service';
import { RendererInitializerService } from '../../renderers/renderer-initializer.service';
import { ElohimNavigatorComponent } from '../../../elohim/components/elohim-navigator/elohim-navigator.component';

@Component({
  selector: 'app-lamad-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, FormsModule, ElohimNavigatorComponent],
  templateUrl: './lamad-layout.component.html',
  styleUrls: ['./lamad-layout.component.css']
})
export class LamadLayoutComponent implements OnInit, OnDestroy {
  isReady = false;
  isHomePage = false;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly router: Router,
    // Injecting RendererInitializerService triggers renderer registration
    private readonly _rendererInit: RendererInitializerService
  ) {}

  ngOnInit(): void {
    // Verify data is loadable by fetching the content index
    this.dataLoader.getContentIndex().pipe(takeUntil(this.destroy$)).subscribe({
      next: () => {
        this.isReady = true;
      },
      error: () => {
        this.isReady = true; // Still mark ready to show error state
      }
    });

    // Track route for UI state
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd),
      takeUntil(this.destroy$)
    ).subscribe(() => {
      this.checkIfHomePage();
    });

    this.checkIfHomePage();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  private checkIfHomePage(): void {
    this.isHomePage = this.router.url === '/lamad' || this.router.url === '/lamad/';
  }
}

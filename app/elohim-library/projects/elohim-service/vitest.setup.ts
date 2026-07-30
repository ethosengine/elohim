import 'zone.js';
import 'zone.js/testing';
import { TestBed } from '@angular/core/testing';
// platformBrowserTesting() (unlike the deprecated platformBrowserDynamicTesting()) does
// NOT pull in the JIT compiler implicitly — specs that JIT-compile injectables/components
// fail with "needs to be compiled using the JIT compiler" without this import.
import '@angular/compiler';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';

// Initialize Angular Testing Environment
TestBed.initTestEnvironment(BrowserTestingModule, platformBrowserTesting());

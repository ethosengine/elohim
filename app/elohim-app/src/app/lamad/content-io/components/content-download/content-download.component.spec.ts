import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ContentDownloadComponent } from './content-download.component';

describe('ContentDownloadComponent', () => {
  let component: ContentDownloadComponent;
  let fixture: ComponentFixture<ContentDownloadComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ContentDownloadComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ContentDownloadComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});

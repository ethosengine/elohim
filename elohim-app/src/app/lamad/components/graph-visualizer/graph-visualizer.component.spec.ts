import { ComponentFixture, TestBed } from '@angular/core/testing';
import { GraphVisualizerComponent } from './graph-visualizer.component';
import { provideRouter } from '@angular/router';

describe('GraphVisualizerComponent', () => {
  let component: GraphVisualizerComponent;
  let fixture: ComponentFixture<GraphVisualizerComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [GraphVisualizerComponent],
      providers: [
        provideRouter([])
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(GraphVisualizerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});

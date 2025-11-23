import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';
import { ContentNode } from '../models/content-node.model';

export interface PathNode {
  node: ContentNode;
  order: number;
  depth: number;
  category: string;
}

@Injectable({
  providedIn: 'root'
})
export class LearningPathService {
  
  // Temporary mock implementation
  // This will need to be fully implemented to calculate orientation and suggest paths
  // based on affinity and graph structure.
  
  private _path: PathNode[] = [];
  private readonly pathSubject = new BehaviorSubject<PathNode[]>([]);

  public readonly path$ = this.pathSubject.asObservable();

  constructor() {}

  setPath(nodes: ContentNode[]) {
     const pathNodes: PathNode[] = nodes.map((node, index) => ({
         node,
         order: index,
         depth: 0, // Flattened for now
         category: node.metadata?.['category'] ?? 'general'
     }));
     this._path = pathNodes;
     this.pathSubject.next(pathNodes);
  }

  getPath(): PathNode[] {
    return this._path;
  }

  getNodePosition(nodeId: string): number {
    return this._path.findIndex(n => n.node.id === nodeId);
  }

  isInPath(nodeId: string): boolean {
    return this._path.some(n => n.node.id === nodeId);
  }

  getNextNode(currentId: string): PathNode | null {
    const index = this._path.findIndex(n => n.node.id === currentId);
    if (index >= 0 && index < this._path.length - 1) {
        return this._path[index + 1];
    }
    return null;
  }

  getPreviousNode(currentId: string): PathNode | null {
    const index = this._path.findIndex(n => n.node.id === currentId);
    if (index > 0) {
        return this._path[index - 1];
    }
    return null;
  }

  getPathProgress(affinityMap: Map<string, number>): number {
      if (this._path.length === 0) return 0;
      
      let totalAffinity = 0;
      this._path.forEach(pn => {
          totalAffinity += affinityMap.get(pn.node.id) ?? 0;
      });
      
      return (totalAffinity / this._path.length) * 100;
  }
}
import os
import json
import re

DOCS_DIR = 'elohim-app/src/assets/docs'
MANIFEST_PATH = 'elohim-app/src/assets/docs/manifest.json'

def get_node_type(file_path):
    if file_path.endswith('.feature'):
        return 'feature'
        
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            
            # Check for explicit node_type
            match = re.search(r'^node_type:\s*(.+)$', content, re.MULTILINE)
            if match:
                return match.group(1).strip()
            
            # Check for user_type (Archetype)
            if re.search(r'^user_type:\s*(.+)$', content, re.MULTILINE):
                return 'user_type'
                
            # Check for epic definition
            if 'epic.md' in file_path or file_path.endswith('manifesto.md'):
                return 'epic'

            return None
    except Exception as e:
        print(f"Error reading {file_path}: {e}")
        return None

def generate_manifest():
    files_list = []
    
    for root, dirs, files in os.walk(DOCS_DIR):
        for file in files:
            if file.endswith('.md') or file.endswith('.feature'):
                if file == 'manifest.json':
                    continue

                full_path = os.path.join(root, file)
                rel_path = os.path.relpath(full_path, DOCS_DIR)
                
                node_type = get_node_type(full_path)
                
                if node_type:
                    files_list.append({
                        'path': rel_path,
                        'type': node_type
                    })
                else:
                    # Fallback for root files or unmatched
                    if file == 'manifesto.md':
                         files_list.append({'path': rel_path, 'type': 'epic'})
                    elif 'global-orchestra' in file or 'hardware-spec' in file or 'observer-protocol' in file:
                         files_list.append({'path': rel_path, 'type': 'epic'})

    # Write manifest
    with open(MANIFEST_PATH, 'w', encoding='utf-8') as f:
        json.dump({'files': files_list}, f, indent=2)
    
    print(f"Generated manifest with {len(files_list)} files.")

if __name__ == '__main__':
    generate_manifest()
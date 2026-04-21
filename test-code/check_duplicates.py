import json
import os
from collections import defaultdict

def check_duplicates(file_path):
    if not os.path.exists(file_path):
        print(f"Error: File {file_path} not found.")
        return

    with open(file_path, 'r', encoding='utf-8') as f:
        try:
            data = json.load(f)
        except json.JSONDecodeError as e:
            print(f"Error decoding JSON: {e}")
            return

    content = data.get('content', [])
    series_map = defaultdict(list)

    for item in content:
        url = item.get('url')
        if url:
            series_map[url].append(item)

    return series_map

def list_duplicates(series_map):
    duplicates = {url: items for url, items in series_map.items() if len(items) > 1}

    if not duplicates:
        print("No duplicate URLs found.")
    else:
        print(f"Found {len(duplicates)} duplicate URLs:\n")
        for url, items in duplicates.items():
            print(f"URL: {url}")
            print(f"Count: {len(items)}")
            for i, item in enumerate(items, 1):
                name = item.get('name', 'N/A')
                title = item.get('metadata', {}).get('title', 'N/A')
                item_id = item.get('id', 'N/A')
                print(f"  {i}. Title: {title} | Name: {name} | ID: {item_id}")
            print("-" * 40)

def generate_sql_cleanup(series_map):
    duplicates = {url: items for url, items in series_map.items() if len(items) > 1}
    if not duplicates:
        return

    print("\n-- SQL CLEANUP STATEMENTS (Keep oldest record only) --")
    for url, items in duplicates.items():
        # Sort by 'created' timestamp. Older dates (smaller strings) come first.
        # Format: 2026-04-18T04:18:12Z
        sorted_items = sorted(items, key=lambda x: x.get('created', ''))
        
        # Keep the first one (oldest), delete the rest
        oldest = sorted_items[0]
        to_delete = sorted_items[1:]

        print(f"\n-- Duplicates for URL: {url}")
        print(f"-- Keeping oldest ID: {oldest.get('id')} (Created: {oldest.get('created')})")
        for item in to_delete:
            item_id = item.get('id')
            print(f'DELETE FROM public."SERIES" WHERE "ID"=\'{item_id}\';')
    print("-" * 40)

if __name__ == "__main__":
    # Check if x.json exists in current dir or test-code/
    json_file = "x.json"
    if not os.path.exists(json_file):
        json_file = "test-code/x.json"
        
    series_map = check_duplicates(json_file)
    if series_map:
        list_duplicates(series_map)
        generate_sql_cleanup(series_map)

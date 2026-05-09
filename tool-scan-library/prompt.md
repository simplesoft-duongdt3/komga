because scan library of komga too slow for big folder (1500 series, 300,000 book) during update new books, add new series

plan a script tool Python:

Input:
+ Read only info of posgrest database production
192.168.1.169
port 5433
database komga
user ai_readonly WITH PASSWORD 'ai_readonly_pass'


+ library id: 0Q3CKC76902B7
+ real folder path of library + inside docker folder path of library: /volume1/Shared/my-library/data-books-audiobooks/Manga_Ebook/Manhwa + /data/data-books-audiobooks/Manga_Ebook/Manhwa
+ a sample of folder library in local for test: /Users/teamcumahay/Downloads/ThienThaiTruyen

Output: folder tool-scan-library
SQL
+ Insert new seires with metadata, thumbnail
+ Update seires with metadata, thumbnail
+ Delete deleted series
+ Insert new books with metadata, thumbnail
+ Update books with metadata, thumbnail
+ Delete deleted books


+ Best performance, multi threading
print "fetching log.txt..."

f = open('log.txt', 'r')

print "storing data..."

list = []

for line in f:
	list.append(line),
	
print "counting most occured..."
	
max_count = 0;
max_item = "";
	
for item in list:
	count = list.count(item)
	if count > max_count:
		max_count = count
		max_item = item,
	
	
print "RESULT"

print "Total items:"
print len(list)
print "Most occured item:"
print max_count
print "String:"
print max_item


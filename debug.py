print "DEBUGGING log.txt"

f = open('log.txt', 'r')
list = []
for line in f:
	list.append(line),
	
max_count1 = 0
max_count2 = 0
max_count3 = 0
max_item1 = ""
max_item2 = ""
max_item3 = ""
	
for item in list:
	count = list.count(item)
	if count > max_count1:
		max_count1 = count
		max_item1 = item
		continue
	if max_item1 != item and count > max_count2:
		max_count2 = count
		max_item2 = item
		continue
	if max_item1 != item and max_item2 != item and count > max_count3:
		max_count3 = count
		max_item3 = item,

print "Total items:"
print len(list)
print "top 3 items:"
print max_count1
print max_item1
print "---"
print max_count2
print max_item2
print "---"
print max_count3
print max_item3
print "---"





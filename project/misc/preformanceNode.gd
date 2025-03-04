extends Node2D

var frames_array = []
const MAX_FRAMES = 100

func _process(delta):
	# Record frame time
	frames_array.push_back(delta)
	if frames_array.size() > MAX_FRAMES:
		frames_array.pop_front()
	queue_redraw()  # Force redraw

func _draw():
	# Draw frame time graph
	if frames_array.size() < 2:
		return
	
	var graph_height = 100
	var max_time = 1.0/30.0  # Anything above 30fps will show as a spike
	
	for i in range(frames_array.size() - 1):
		var p1 = Vector2(i * 5, graph_height * (1.0 - frames_array[i] / max_time))
		var p2 = Vector2((i + 1) * 5, graph_height * (1.0 - frames_array[i+1] / max_time))
		
		# Red if below target framerate
		var color = Color.GREEN
		if frames_array[i+1] > 1.0/60.0:
			color = Color.RED
		
		draw_line(p1, p2, color, 2.0)
	
	# Draw 60fps and 30fps lines
	draw_line(Vector2(0, graph_height * (1.0 - 1.0/60.0/max_time)), 
			 Vector2(MAX_FRAMES * 5, graph_height * (1.0 - 1.0/60.0/max_time)), 
			 Color(0,1,0,0.5), 1.0)
	
	draw_line(Vector2(0, graph_height * (1.0 - 1.0/30.0/max_time)), 
			 Vector2(MAX_FRAMES * 5, graph_height * (1.0 - 1.0/30.0/max_time)), 
			 Color(1,0,0,0.5), 1.0)

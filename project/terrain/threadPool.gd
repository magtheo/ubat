extends Node

const NUM_WORKERS = 4       # Or 4, depending on CPU
var tasks = []
var tasks_mutex = Mutex.new()
var workers = []
var libchunk_generator = null

func _ready():
	# Create a fixed number of threads
	for i in range(NUM_WORKERS):
		var t = Thread.new()
		var ret = t.start(_worker_loop.bind(i))  # pass self, string method, then your argument
		if ret != OK:
			push_error("Failed to start worker thread " + str(i))
		workers.append(t)

# Called by your terrain code: queue a "chunk generation" job
func enqueue_chunk(cx, cy, CHUNK_SIZE):
	tasks_mutex.lock()
	tasks.append({"cx": cx, "cy": cy, "CHUNK_SIZE": CHUNK_SIZE})
	tasks_mutex.unlock()

func _worker_loop(_worker_id):
	while true:
		tasks_mutex.lock()

		if tasks.size() > 0:
			var task = tasks.pop_front()
			tasks_mutex.unlock()

			# Do the heavy chunk generation off-thread:
			var chunk = null
			if libchunk_generator:
				var biome_data = libchunk_generator.generate_biome_data(task.cx, task.cy, task.CHUNK_SIZE) # where should the biomeData be stored
				
				chunk = libchunk_generator.generate_chunk_with_biome_data(task.cx, task.cy, biome_data)

			# Defer final scene add to the main thread
			call_deferred("_finalize_chunk", chunk, task.cx, task.cy)

		else:
			# No tasks, so unlock and sleep a bit
			tasks_mutex.unlock()
			await get_tree().create_timer(0.01).timeout  # or OS.delay_msec(10) in older versions

func _finalize_chunk(chunk, cx, cy):
	if chunk:
		# "Owner" node would be your terrain manager or root
		get_parent().add_child(chunk)
		# You might store "loaded_chunks[Vector2i(cx, cy)] = true" here
		print("ThreadPool: ✅ Chunk at (%d, %d) finalized." % [cx, cy])

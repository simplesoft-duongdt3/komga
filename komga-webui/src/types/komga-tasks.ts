export interface TaskDto {
  id: string,
  simpleType: string,
  status: TaskStatus,
  owner?: string,
  priority: number,
  groupId?: string,
  createdDate: string,
  lastModifiedDate: string,
  durationMillis: number,
}

export type TaskStatus = 'QUEUED' | 'RUNNING'

export interface TaskFilter {
  status?: TaskStatus,
  simpleType?: string[],
}
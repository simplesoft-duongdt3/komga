import {AxiosInstance} from 'axios'
import {TaskDto, TaskFilter} from '@/types/komga-tasks'

const qs = require('qs')

const API_TASKS = '/api/v1/tasks'

export default class KomgaTasksService {
  private http: AxiosInstance

  constructor(http: AxiosInstance) {
    this.http = http
  }

  async getAll(pageRequest?: PageRequest, filter?: TaskFilter): Promise<Page<TaskDto>> {
    try {
      const params = {...pageRequest} as any
      if (filter?.status) params.status = filter.status
      if (filter?.simpleType?.length) params.simpleType = filter.simpleType

      return (await this.http.get(API_TASKS, {
        params: params,
        paramsSerializer: params => qs.stringify(params, {indices: false}),
      })).data
    } catch (e) {
      let msg = 'An error occurred while trying to retrieve tasks'
      if (e.response.data.message) {
        msg += `: ${e.response.data.message}`
      }
      throw new Error(msg)
    }
  }

  async deleteAllTasks(): Promise<number> {
    try {
      return (await this.http.delete(API_TASKS)).data
    } catch (e) {
      let msg = 'An error occurred while trying to delete all tasks'
      if (e.response.data.message) {
        msg += `: ${e.response.data.message}`
      }
      throw new Error(msg)
    }
  }
}
